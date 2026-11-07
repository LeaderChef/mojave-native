import Foundation
import CryptoKit

func runSelfTests() throws {
    var checks = 0
    func check(_ condition: @autoclosure () throws -> Bool, _ name: String) throws {
        guard try condition() else { throw LauncherFailure.invalid(name) }
        checks += 1; print("PASS \(name)")
    }
    func rejects(_ name: String, _ body: () throws -> Void) throws {
        do { try body() } catch { checks += 1; print("PASS \(name)"); return }
        throw LauncherFailure.invalid("Expected rejection: \(name)")
    }
    let beforeProfile = "[Settings]\nvolume-master=0.02\nHUD-SCALE=0.9\n[Controls]\nfps-counter=0\nForward=W\n"
    let profile = MacProfileDefaults.fillingMissing(in: beforeProfile, defaults: "[Settings]\nvolume-master=0.05\nhud-scale=0.8\nfps-counter=1\n")
    try check(profile.contains("volume-master=0.02\nHUD-SCALE=0.9\nfps-counter=1\n[Controls]\nfps-counter=0\nForward=W"), "Mac defaults retain player choices and put missing settings in the correct section")
    try check(MacProfileDefaults.fillingMissing(in: profile, defaults: "[Settings]\nvolume-master=0.05\nhud-scale=0.8\nfps-counter=1\n") == profile, "Mac defaults are idempotent")
    // Exercise the same initialization as the GUI in the real bundle domain;
    // no values are written. The previous own-identifier suite trapped here.
    _ = LauncherPreferences.store
    try check(true, "actual app preferences initialize without an own-bundle suite trap")
    let base = URL(fileURLWithPath: ProcessInfo.processInfo.environment["J6_TEST_TMP"] ?? NSTemporaryDirectory(), isDirectory: true)
    let root = base.appendingPathComponent("mojave-launcher-test-\(UUID().uuidString)", isDirectory: true)
    try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
    defer { try? FileManager.default.removeItem(at: root) }
    let game = root.appendingPathComponent("Owned Game; literal path/Data", isDirectory: true)
    try FileManager.default.createDirectory(at: game, withIntermediateDirectories: true)
    try check(!GameFiles.inspect(nil).valid, "no chosen data is unavailable")
    try check(!GameFiles.inspect(game).valid, "empty folder is rejected")
    for file in GameFiles.required { try Data((file.hasSuffix("esm") ? "TES4synthetic test record" : "BSAsynthetic test archive").utf8).write(to: game.appendingPathComponent(file)) }
    try check(GameFiles.inspect(game).valid, "complete synthetic base layout is accepted")
    try check(GameFiles.inspect(game.deletingLastPathComponent()).directory == game, "root chooser resolves child Data")
    for file in GameFiles.dlcs { try Data("TES4synthetic DLC fixture".utf8).write(to: game.appendingPathComponent(file)) }
    try check(GameFiles.inspect(game).dlcCount == 9, "nine official add-on masters are counted from files")
    try Data("WRONG".utf8).write(to: game.appendingPathComponent("FalloutNV.esm"))
    try check(!GameFiles.inspect(game).valid, "bad master header is rejected")
    try Data("TES4fixture".utf8).write(to: game.appendingPathComponent("FalloutNV.esm"))
    try Data().write(to: game.appendingPathComponent("Update.bsa"))
    try check(GameFiles.inspect(game).missing == ["Update.bsa"], "empty archive is identified by actual filename")
    try Data("BSAfixture".utf8).write(to: game.appendingPathComponent("Update.bsa"))

    var arm = Data([0xcf, 0xfa, 0xed, 0xfe, 0x0c, 0x00, 0x00, 0x01, 0, 0, 0, 0, 2, 0, 0, 0])
    arm.append(Data(repeating: 0, count: 48))
    try check(Integrity.arm64Executable(arm), "ARM64 executable Mach-O header is recognized")
    var x86 = arm; x86[4] = 7
    try check(!Integrity.arm64Executable(x86), "Intel-only header is rejected")
    var library = arm; library[12] = 6
    try check(!Integrity.arm64Executable(library), "ARM64 dynamic library is not an executable")
    try check(!Integrity.arm64Executable(Data([1, 2, 3])), "truncated binary is rejected")
    var fat = Data([0xca, 0xfe, 0xba, 0xbe, 0, 0, 0, 1, 1, 0, 0, 12, 0, 0, 0, 0, 0, 0, 0, 32, 0, 0, 0, 64, 0, 0, 0, 0, 0, 0, 0, 0]); fat.append(arm)
    try check(Integrity.arm64Executable(fat), "universal binary validates its actual ARM64 slice")
    fat[19] = 250
    try check(!Integrity.arm64Executable(fat), "out-of-bounds universal slice is rejected")
    let binary = root.appendingPathComponent("Runtime/nvforever")
    try FileManager.default.createDirectory(at: binary.deletingLastPathComponent(), withIntermediateDirectories: true)
    try arm.write(to: binary)
    try FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: binary.path)
    let hash = try Integrity.sha256(binary)
    try Integrity.verifyBinary(binary, expectedHash: hash)
    try check(hash.count == 64, "SHA256 of actual fixture bytes verifies")
    try rejects("missing pinned binary hash disables launch") { try Integrity.verifyBinary(binary, expectedHash: nil) }
    try rejects("mismatched binary hash disables launch") { try Integrity.verifyBinary(binary, expectedHash: String(repeating: "0", count: 64)) }
    let schemas = root.appendingPathComponent("Runtime/schemas")
    for part in ["fnv", "nif"] { try FileManager.default.createDirectory(at: schemas.appendingPathComponent(part), withIntermediateDirectories: true); try Data("synthetic schema fixture".utf8).write(to: schemas.appendingPathComponent(part + "/example.json")) }
    let catalogue = schemas.appendingPathComponent("script/functions.json")
    try FileManager.default.createDirectory(at: catalogue.deletingLastPathComponent(), withIntermediateDirectories: true)
    let catalogueBytes = Data("{\"functions\":[]}".utf8)
    try catalogueBytes.write(to: catalogue)
    let home = root.appendingPathComponent("Profile")
    let plan = try LaunchPlan.make(binary: binary, expectedHash: hash, schemas: schemas, data: game, home: home,
                                   environment: ["HOME": "/preserved", "NVF_AUTO_ACTION": "bad", "NVFOREVER_DATA": "/wrong", "DYLD_INSERT_LIBRARIES": "bad", "__CFBundleIdentifier": "local.j6.mojave-relay", "SAFE_VALUE": "okay", "NVF_PLAY_SCRIPT_FUNCTIONS": "/untrusted/catalogue.json"])
    try check(plan.arguments.contains(game.path), "chosen data path stays one literal process argument")
    try check(plan.arguments.suffix(4) == ["--size", "1280x800", "--window", "windowed"], "launch arguments are fixed to the native main menu")
    try check(plan.environment["HOME"] == "/preserved" && plan.environment["NVFOREVER_HOME"] == home.path && plan.environment["NVF_AUTO_ACTION"] == nil && plan.environment["DYLD_INSERT_LIBRARIES"] == nil, "runtime profile is isolated and inherited automation hooks removed")
    try check(plan.environment["NVF_PLAY_SCRIPT_FUNCTIONS"] == catalogue.path, "script runtime uses the bundled catalogue instead of an inherited or build-machine path")
    try check(!FileManager.default.fileExists(atPath: home.path), "launch planning does not create user folders")
    try check(plan.environment["__CFBundleIdentifier"] == nil && plan.environment["SAFE_VALUE"] == "okay", "game does not inherit the launcher's LaunchServices bundle identity")
    try rejects("profile overlapping game data is refused") { _ = try LaunchPlan.make(binary: binary, expectedHash: hash, schemas: schemas, data: game, home: game.appendingPathComponent("profile"), environment: [:]) }
    try rejects("missing schemas refuse launch") { _ = try LaunchPlan.make(binary: binary, expectedHash: hash, schemas: root.appendingPathComponent("missing"), data: game, home: home, environment: [:]) }
    try FileManager.default.removeItem(at: catalogue)
    try rejects("missing script catalogue refuses launch before profile creation") { _ = try LaunchPlan.make(binary: binary, expectedHash: hash, schemas: schemas, data: game, home: home, environment: [:]) }
    try catalogueBytes.write(to: catalogue)
    let moved = root.appendingPathComponent("Relocated bundle/runtime")
    try FileManager.default.createDirectory(at: moved.deletingLastPathComponent(), withIntermediateDirectories: true)
    try FileManager.default.copyItem(at: binary.deletingLastPathComponent(), to: moved)
    let relocatedPlan = try LaunchPlan.make(binary: moved.appendingPathComponent("nvforever"), expectedHash: hash, schemas: moved.appendingPathComponent("schemas"), data: game, home: home, environment: [:])
    try check(relocatedPlan.environment["NVF_PLAY_SCRIPT_FUNCTIONS"] == moved.appendingPathComponent("schemas/script/functions.json").path && relocatedPlan.environment["NVF_PLAY_SCRIPT_FUNCTIONS"] != catalogue.path, "relocated bundle points scripts at its relocated catalogue")
    try check(BridgeCommand(rawValue: "runShell") == nil && BridgeCommand(rawValue: "play") == .play && BridgeCommand.allCases.count == 5, "native command bridge has exactly five allowed actions")

    let privateKey = Curve25519.Signing.PrivateKey()
    let asset = UpdateAsset(platform: "aarch64-apple-darwin", url: "https://updates.example.invalid/nvforever", sha256: hash, bytes: Int64(arm.count))
    let manifest = UpdateManifest(version: "test-only", sequence: 2, assets: [asset])
    func envelope(_ value: UpdateManifest) throws -> Data {
        let payload = try JSONEncoder().encode(value)
        return try JSONEncoder().encode(SignedUpdate(payload: payload.base64EncodedString(), signature: try privateKey.signature(for: payload).base64EncodedString()))
    }
    let signed = try envelope(manifest)
    let key = privateKey.publicKey.rawRepresentation
    let verified = try UpdateVerifier.verify(envelope: signed, pinnedKey: key, allowedHost: "updates.example.invalid", minimumSequence: 1)
    try check(verified.0.sequence == 2 && verified.1.platform == "aarch64-apple-darwin", "actual CryptoKit Ed25519 signature verifies raw manifest payload")
    try rejects("unconfigured update channel never reports current") { _ = try UpdateVerifier.verify(envelope: signed, pinnedKey: nil, allowedHost: nil, minimumSequence: 0) }
    try rejects("wrong signing key is rejected") { _ = try UpdateVerifier.verify(envelope: signed, pinnedKey: Curve25519.Signing.PrivateKey().publicKey.rawRepresentation, allowedHost: "updates.example.invalid", minimumSequence: 0) }
    try rejects("rollback and replay sequence is rejected") { _ = try UpdateVerifier.verify(envelope: signed, pinnedKey: key, allowedHost: "updates.example.invalid", minimumSequence: 2) }
    try rejects("non-pinned update host is rejected") { _ = try UpdateVerifier.verify(envelope: signed, pinnedKey: key, allowedHost: "other.example.invalid", minimumSequence: 0) }
    let insecureAsset = UpdateAsset(platform: asset.platform, url: "http://updates.example.invalid/file", sha256: hash, bytes: Int64(arm.count))
    try rejects("signed HTTP asset still rejected") { _ = try UpdateVerifier.verify(envelope: try envelope(UpdateManifest(version: "test", sequence: 3, assets: [insecureAsset])), pinnedKey: key, allowedHost: "updates.example.invalid", minimumSequence: 0) }
    try UpdateVerifier.verifyDownloaded(binary, asset: asset)
    try check(true, "downloaded bytes require exact size, digest and ARM64 type")
    try Data("tampered".utf8).write(to: binary)
    try rejects("tampered downloaded binary is rejected") { try UpdateVerifier.verifyDownloaded(binary, asset: asset) }
    print("SELF-TEST OK: \(checks) checks; no NSApplication, no window, no game launch, no network, no real user preferences written.")
}
