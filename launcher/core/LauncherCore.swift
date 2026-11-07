import Foundation
import CryptoKit

struct LauncherConfig: Codable {
    var name: String
    var version: String
    var binaryRelativePath: String
    var schemasRelativePath: String
    var binarySHA256: String?
    var developmentBinaryRelativePath: String
    var developmentSchemasRelativePath: String
    var dataDefaultHint: String
    var updateManifestURL: String?
    var updatePublicKeyBase64: String?
    var updateAssetHost: String?
}

enum LauncherFailure: Error, CustomStringConvertible {
    case invalid(String)
    var description: String { switch self { case .invalid(let text): return text } }
}

enum LauncherPreferences {
    // The current application's domain is already standard. Opening its own
    // bundle identifier as a separate suite is invalid on current macOS.
    static var store: UserDefaults { .standard }
}

struct DataInspection {
    let directory: URL?
    let missing: [String]
    let dlcCount: Int
    let error: String?
    var valid: Bool { directory != nil && missing.isEmpty && error == nil }
}

enum GameFiles {
    static let required = ["FalloutNV.esm", "Fallout - Textures.bsa", "Fallout - Textures2.bsa", "Fallout - Meshes.bsa", "Fallout - Voices1.bsa", "Fallout - Sound.bsa", "Fallout - Misc.bsa", "Update.bsa"]
    static let dlcs = ["DeadMoney.esm", "HonestHearts.esm", "OldWorldBlues.esm", "LonesomeRoad.esm", "GunRunnersArsenal.esm", "ClassicPack.esm", "MercenaryPack.esm", "TribalPack.esm", "CaravanPack.esm"]

    static func inspect(_ selection: URL?) -> DataInspection {
        guard let selection else { return DataInspection(directory: nil, missing: required, dlcCount: 0, error: nil) }
        let fm = FileManager.default
        let selected = selection.standardizedFileURL.resolvingSymlinksInPath()
        var folder = selected
        if let names = try? fm.contentsOfDirectory(atPath: folder.path), !names.contains(where: { $0.lowercased() == "falloutnv.esm" }), let child = names.first(where: { $0.lowercased() == "data" }) {
            folder = folder.appendingPathComponent(child, isDirectory: true)
        }
        guard let files = try? fm.contentsOfDirectory(at: folder, includingPropertiesForKeys: [.isRegularFileKey, .fileSizeKey]) else {
            return DataInspection(directory: nil, missing: required, dlcCount: 0, error: "Bu klasör okunamıyor.")
        }
        var found: [String: URL] = [:]
        for file in files {
            let key = file.lastPathComponent.lowercased()
            guard found[key] == nil else { return DataInspection(directory: folder, missing: [], dlcCount: 0, error: "Aynı adlı dosyalar farklı harflerle yineleniyor: \(file.lastPathComponent)") }
            found[key] = file
        }
        func realFile(_ name: String) -> URL? {
            guard let candidate = found[name.lowercased()] else { return nil }
            let path = candidate.resolvingSymlinksInPath()
            guard fm.isReadableFile(atPath: path.path), let values = try? path.resourceValues(forKeys: [.isRegularFileKey, .fileSizeKey]), values.isRegularFile == true, (values.fileSize ?? 0) > 0 else { return nil }
            return path
        }
        let missing = required.filter { realFile($0) == nil }
        var error: String?
        if let esm = realFile("FalloutNV.esm"), let handle = try? FileHandle(forReadingFrom: esm) {
            let header = try? handle.read(upToCount: 4); try? handle.close()
            if header != Data("TES4".utf8) { error = "FalloutNV.esm başlığı geçerli değil." }
        }
        return DataInspection(directory: folder, missing: missing, dlcCount: dlcs.filter { realFile($0) != nil }.count, error: error)
    }
}

enum Integrity {
    static func sha256(_ file: URL) throws -> String {
        let handle = try FileHandle(forReadingFrom: file)
        defer { try? handle.close() }
        var hasher = SHA256()
        while let block = try handle.read(upToCount: 1024 * 1024), !block.isEmpty { hasher.update(data: block) }
        return hasher.finalize().map { String(format: "%02x", $0) }.joined()
    }

    // Mach-O headers documented by Apple's public loader.h/fat.h; no executable code is inspected.
    static func arm64Executable(_ bytes: Data) -> Bool {
        func u32(_ at: Int, little: Bool) -> UInt32? {
            guard at >= 0, bytes.count >= 4, at <= bytes.count - 4 else { return nil }
            var value: UInt32 = 0
            for index in 0..<4 {
                let shift = (little ? index : 3 - index) * 8
                value |= UInt32(bytes[at + index]) << shift
            }
            return value
        }
        func thin(_ at: Int) -> Bool { u32(at, little: true) == 0xfeedfacf && u32(at + 4, little: true) == 0x0100000c && u32(at + 12, little: true) == 2 }
        if thin(0) { return true }
        guard let magic = u32(0, little: false), magic == 0xcafebabe || magic == 0xcafebabf, let count = u32(4, little: false), count > 0, count <= 64 else { return false }
        let stride = magic == 0xcafebabf ? 32 : 20
        for i in 0..<Int(count) {
            let at = 8 + i * stride
            guard at + stride <= bytes.count else { return false }
            if u32(at, little: false) != 0x0100000c { continue }
            let offset: UInt64
            if stride == 32 {
                guard let hi = u32(at + 8, little: false), let lo = u32(at + 12, little: false) else { return false }
                offset = UInt64(hi) << 32 | UInt64(lo)
            } else { offset = UInt64(u32(at + 8, little: false) ?? UInt32.max) }
            if offset <= UInt64(Int.max), thin(Int(offset)) { return true }
        }
        return false
    }

    static func verifyBinary(_ path: URL, expectedHash: String?) throws {
        guard FileManager.default.isExecutableFile(atPath: path.path) else { throw LauncherFailure.invalid("Oyun motoru henüz paketlenmedi.") }
        guard let expectedHash, expectedHash.count == 64, expectedHash.allSatisfy({ $0.isHexDigit }) else { throw LauncherFailure.invalid("Motorun doğrulama kaydı henüz hazırlanmadı.") }
        guard try sha256(path) == expectedHash.lowercased() else { throw LauncherFailure.invalid("Motor dosyası doğrulama kaydıyla eşleşmiyor.") }
        // Universal binaries may keep an ARM slice beyond the header, so read the actual file.
        guard arm64Executable(try Data(contentsOf: path, options: .mappedIfSafe)) else { throw LauncherFailure.invalid("Motor Apple Silicon ARM64 çalıştırılabilir dosyası değil.") }
    }

    static func verifySchemas(_ schemas: URL) throws {
        for part in ["fnv", "nif"] {
            let folder = schemas.appendingPathComponent(part)
            var isDirectory: ObjCBool = false
            guard FileManager.default.fileExists(atPath: folder.path, isDirectory: &isDirectory), isDirectory.boolValue,
                  let contents = try? FileManager.default.contentsOfDirectory(atPath: folder.path), !contents.isEmpty else {
                throw LauncherFailure.invalid("Motor şemaları eksik: \(part)")
            }
        }
        let catalogue = schemas.appendingPathComponent("script/functions.json")
        guard let bytes = try? Data(contentsOf: catalogue), !bytes.isEmpty else {
            throw LauncherFailure.invalid("Oyun betiklerinin işlev listesi eksik.")
        }
    }
}

struct LaunchPlan {
    let executable: URL
    let arguments: [String]
    let environment: [String: String]
    let home: URL
    let logs: URL

    static func make(binary: URL, expectedHash: String?, schemas: URL, data: URL?, home: URL, environment: [String: String]) throws -> LaunchPlan {
        let inspection = GameFiles.inspect(data)
        guard inspection.valid, let data = inspection.directory else {
            throw LauncherFailure.invalid(inspection.error ?? "Oyun verileri eksik: \(inspection.missing.joined(separator: ", "))")
        }
        try Integrity.verifyBinary(binary, expectedHash: expectedHash)
        try Integrity.verifySchemas(schemas)
        let cleanHome = home.standardizedFileURL.resolvingSymlinksInPath()
        let cleanData = data.standardizedFileURL.resolvingSymlinksInPath()
        func overlaps(_ a: URL, _ b: URL) -> Bool { a.path == b.path || a.path.hasPrefix(b.path + "/") || b.path.hasPrefix(a.path + "/") }
        guard !overlaps(cleanHome, cleanData), !overlaps(cleanHome, binary.deletingLastPathComponent()) else { throw LauncherFailure.invalid("Oyun verisi, motor ve kayıt klasörleri ayrı olmalı.") }
        var env = environment.filter { !$0.key.hasPrefix("NVF_") && !$0.key.hasPrefix("NVFOREVER_") && !$0.key.hasPrefix("DYLD_") && $0.key != "LD_PRELOAD" && $0.key != "__CFBundleIdentifier" }
        env["NVFOREVER_HOME"] = cleanHome.path
        env["NVF_PLAY_SCRIPT_FUNCTIONS"] = schemas.standardizedFileURL.resolvingSymlinksInPath().appendingPathComponent("script/functions.json").path
        return LaunchPlan(executable: binary, arguments: ["--data", cleanData.path, "--esm-schema", schemas.appendingPathComponent("fnv").path, "--nif-schema", schemas.appendingPathComponent("nif").path, "--play", "--menu", "--size", "1280x800", "--window", "windowed"], environment: env, home: cleanHome, logs: cleanHome.appendingPathComponent("logs"))
    }
}

// This verifier is a protocol foundation, not a configured download/install service.
// The signed payload bytes are verified before decoding, avoiding JSON canonicalization ambiguity.
struct SignedUpdate: Codable { let payload: String; let signature: String }
struct UpdateAsset: Codable { let platform: String; let url: String; let sha256: String; let bytes: Int64 }
struct UpdateManifest: Codable { let version: String; let sequence: UInt64; let assets: [UpdateAsset] }
enum UpdateVerifier {
    static func verify(envelope: Data, pinnedKey: Data?, allowedHost: String?, minimumSequence: UInt64) throws -> (UpdateManifest, UpdateAsset) {
        guard let pinnedKey, let allowedHost, !allowedHost.isEmpty else { throw LauncherFailure.invalid("Güncelleme kanalı hazırlanıyor.") }
        guard envelope.count <= 128 * 1024 else { throw LauncherFailure.invalid("Güncelleme bildirimi çok büyük.") }
        let signed = try JSONDecoder().decode(SignedUpdate.self, from: envelope)
        guard let payload = Data(base64Encoded: signed.payload), let signature = Data(base64Encoded: signed.signature), signature.count == 64 else { throw LauncherFailure.invalid("Güncelleme imzası eksik.") }
        let key = try Curve25519.Signing.PublicKey(rawRepresentation: pinnedKey)
        guard key.isValidSignature(signature, for: payload) else { throw LauncherFailure.invalid("Güncelleme imzası doğrulanamadı.") }
        let manifest = try JSONDecoder().decode(UpdateManifest.self, from: payload)
        guard manifest.sequence > minimumSequence else { throw LauncherFailure.invalid("Eski veya yinelenen güncelleme reddedildi.") }
        let assets = manifest.assets.filter { $0.platform == "aarch64-apple-darwin" }
        guard assets.count == 1, let asset = assets.first, let url = URL(string: asset.url), url.scheme == "https", url.host == allowedHost, url.user == nil, url.password == nil, url.port == nil || url.port == 443,
              asset.bytes > 0, asset.bytes <= 1024 * 1024 * 1024, asset.sha256.count == 64, asset.sha256.allSatisfy({ $0.isHexDigit }) else { throw LauncherFailure.invalid("Güncelleme platformu, adresi veya dosya kaydı geçersiz.") }
        return (manifest, asset)
    }

    static func verifyDownloaded(_ file: URL, asset: UpdateAsset) throws {
        let size = try file.resourceValues(forKeys: [.fileSizeKey]).fileSize ?? -1
        guard Int64(size) == asset.bytes, try Integrity.sha256(file) == asset.sha256.lowercased() else { throw LauncherFailure.invalid("İndirilen dosya boyutu veya özeti eşleşmiyor.") }
        guard Integrity.arm64Executable(try Data(contentsOf: file, options: .mappedIfSafe)) else { throw LauncherFailure.invalid("Güncelleme ARM64 motor dosyası değil.") }
    }
}

enum BridgeCommand: String, CaseIterable { case status, chooseData, play, openLogs, checkUpdates }

enum MacProfileDefaults {
    /// Fill only missing Settings keys. Controls and choices already made by the
    /// player are retained verbatim, even if they differ from our initial preset.
    static func fillingMissing(in text: String, defaults: String) -> String {
        func entries(_ input: String) -> [(String, String)] {
            var section = ""; var result: [(String, String)] = []
            for line in input.components(separatedBy: .newlines) {
                let t = line.trimmingCharacters(in: .whitespaces)
                if t.hasPrefix("[") && t.hasSuffix("]") { section = String(t.dropFirst().dropLast()).lowercased() }
                else if section == "settings", !t.hasPrefix(";"), !t.hasPrefix("#"), let eq = t.firstIndex(of: "=") {
                    result.append((String(t[..<eq]).trimmingCharacters(in: .whitespaces).lowercased(), line))
                }
            }
            return result
        }
        var seen = Set(entries(text).map { $0.0 })
        let missing = entries(defaults).compactMap { key, line in seen.insert(key).inserted ? line : nil }
        guard !missing.isEmpty else { return text }
        var lines = text.components(separatedBy: "\n")
        if let header = lines.firstIndex(where: { $0.trimmingCharacters(in: .whitespacesAndNewlines).lowercased() == "[settings]" }) {
            let end = lines.indices.dropFirst(header + 1).first { lines[$0].trimmingCharacters(in: .whitespacesAndNewlines).hasPrefix("[") } ?? lines.count
            lines.insert(contentsOf: missing, at: end)
        } else {
            lines.append(contentsOf: ["", "[Settings]"] + missing)
        }
        return lines.joined(separator: "\n") + "\n"
    }
}

struct ResourceLocation {
    let resources: URL
    let workspace: URL?
    let config: LauncherConfig
    var binary: URL { resolve(config.binaryRelativePath, development: config.developmentBinaryRelativePath) }
    var schemas: URL { resolve(config.schemasRelativePath, development: config.developmentSchemasRelativePath) }
    var defaultData: URL? { workspace.map { $0.appendingPathComponent(config.dataDefaultHint) } }
    private func resolve(_ bundled: String, development: String) -> URL {
        let candidate = resources.appendingPathComponent(bundled)
        if FileManager.default.fileExists(atPath: candidate.path) { return candidate }
        return workspace?.appendingPathComponent(development) ?? candidate
    }
    static func load(resources: URL) throws -> ResourceLocation {
        let config = try JSONDecoder().decode(LauncherConfig.self, from: Data(contentsOf: resources.appendingPathComponent("runtime.json")))
        var candidate = resources
        var workspace: URL?
        for _ in 0..<10 {
            if FileManager.default.fileExists(atPath: candidate.appendingPathComponent("work/fnv-native-mac").path) { workspace = candidate; break }
            let parent = candidate.deletingLastPathComponent()
            if parent.path == candidate.path { break }; candidate = parent
        }
        return ResourceLocation(resources: resources, workspace: workspace, config: config)
    }
}
