import Foundation

do {
    try runSelfTests()
} catch {
    fputs("Launcher core checks failed: \(error)\n", stderr)
    exit(1)
}
