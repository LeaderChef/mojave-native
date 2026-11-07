import Foundation
import CoreGraphics
import ImageIO
import UniformTypeIdentifiers

// Timeline/compositing for original project art. This does not edit game assets.
guard CommandLine.arguments.count == 4 else {
    fatalError("usage: render INPUT.png OUTPUT.gif PREVIEW.png")
}
let sourceURL = URL(fileURLWithPath: CommandLine.arguments[1])
guard let source = CGImageSourceCreateWithURL(sourceURL as CFURL, nil),
      let sourceImage = CGImageSourceCreateImageAtIndex(source, 0, nil) else {
    fatalError("Cannot read the original key art")
}
let width = 768, height = 432, frameCount = 96
let frameDelay = 0.08
let colorSpace = CGColorSpaceCreateDeviceRGB()
let bitmapInfo = CGBitmapInfo.byteOrder32Big.rawValue | CGImageAlphaInfo.premultipliedLast.rawValue
var base = [UInt8](repeating: 0, count: width * height * 4)
base.withUnsafeMutableBytes { bytes in
    let context = CGContext(data: bytes.baseAddress, width: width, height: height,
        bitsPerComponent: 8, bytesPerRow: width * 4, space: colorSpace,
        bitmapInfo: bitmapInfo)!
    context.interpolationQuality = .high
    context.draw(sourceImage, in: CGRect(x: 0, y: 0, width: width, height: height))
}

func makeImage(_ bytes: [UInt8]) -> CGImage {
    let provider = CGDataProvider(data: Data(bytes) as CFData)!
    return CGImage(width: width, height: height, bitsPerComponent: 8,
        bitsPerPixel: 32, bytesPerRow: width * 4, space: colorSpace,
        bitmapInfo: CGBitmapInfo(rawValue: bitmapInfo), provider: provider,
        decode: nil, shouldInterpolate: true, intent: .defaultIntent)!
}

let outputURL = URL(fileURLWithPath: CommandLine.arguments[2])
guard let destination = CGImageDestinationCreateWithURL(outputURL as CFURL,
    UTType.gif.identifier as CFString, frameCount, nil) else { fatalError("Cannot create GIF") }
CGImageDestinationSetProperties(destination,
    [kCGImagePropertyGIFDictionary: [kCGImagePropertyGIFLoopCount: 0]] as CFDictionary)
let frameProperties = [kCGImagePropertyGIFDictionary: [
    kCGImagePropertyGIFDelayTime: frameDelay,
    kCGImagePropertyGIFUnclampedDelayTime: frameDelay
]] as CFDictionary
let tau = 2 * Double.pi
var darkPreview: CGImage?
var litPreview: CGImage?

func clamp(_ value: Double) -> Double { max(0, min(1, value)) }

// A visible, slow relighting sequence. The first and last frames are fully lit.
// During the middle, each letter's existing neon is switched back on in order.
func signPower(frame: Int, horizontal: Double, letters: Bool) -> Double {
    if frame < 16 { return 1 }
    if frame < 24 { return 1 - Double(frame - 16) / 8 }
    if frame < 32 { return 0 }
    if frame < 64 {
        if !letters { return 0.25 + 0.75 * Double(frame - 32) / 32 }
        let column = floor(clamp((horizontal - 0.375) / 0.515) * 12.999)
        return clamp((Double(frame - 32) - column * 1.9) / 3)
    }
    return 0.95 + 0.05 * cos(Double(frame - 64) * tau / 32)
}

for frame in 0..<frameCount {
    autoreleasepool {
        var pixels = base
        let phase = tau * Double(frame) / Double(frameCount)
        for y in 0..<height {
            let ny = Double(y) / Double(height)
            for x in 0..<width {
                let nx = Double(x) / Double(width)
                let i = (y * width + x) * 4
                let r = Double(base[i]) / 255
                let b = Double(base[i+2]) / 255
                let warm = clamp((r - b - 0.08) * 5) * clamp((r - 0.22) * 4)
                var gain = 1.0
                // Perspective-aligned lettering, including its amber bloom.
                let textTop = 0.465 - 0.32 * (nx - 0.38)
                let letters = nx > 0.37 && nx < 0.895
                    && ny > textTop - 0.015 && ny < textTop + 0.17
                let frontSign = nx > 0.34 && nx < 0.995
                    && ny > 0.19 && ny < 0.72
                if frontSign {
                    let power = signPower(frame: frame, horizontal: nx, letters: letters)
                    gain = 1 - warm * (1 - power) * (letters ? 0.94 : 0.80)
                }
                // Bright chasing sectors run around the existing Lucky 38 bulbs.
                let dx = (nx - 0.128) / 0.076
                let dy = (ny - 0.474) / 0.132
                let radius = sqrt(dx*dx + dy*dy)
                if radius > 0.80 && radius < 1.17 {
                    let angle = atan2(dy, dx)
                    let chase = pow(max(0, cos(3 * angle - phase * 3)), 4)
                    gain = 1 - warm * (1 - chase) * 0.83
                }
                // Tower ring and entrance lights dim and recover at a different pace.
                if (nx > 0.19 && nx < 0.43 && ny > 0.06 && ny < 0.205)
                    || (nx > 0.08 && nx < 0.35 && ny > 0.635 && ny < 0.81) {
                    let pulse = 0.55 + 0.45 * cos(phase)
                    gain = 1 - warm * (1 - pulse) * 0.75
                }
                for c in 0..<3 {
                    pixels[i+c] = UInt8(max(0, min(255, (Double(base[i+c]) * gain).rounded())))
                }
            }
        }
        // Tiny drifting motes, each fading before its loop wraps.
        for n in 0..<26 {
            let travel = (Double(frame) / Double(frameCount) + Double(n) * 0.61803398875)
                .truncatingRemainder(dividingBy: 1)
            let sx = Double((n * 173 + 31) % width)
            let sy = Double((n * 97 + 11) % height)
            let px = Int((sx + travel * 30).truncatingRemainder(dividingBy: Double(width)))
            let py = Int((sy + 2 * sin(phase + Double(n))).rounded())
            if py >= 0 && py < height {
                let i = (py * width + px) * 4
                let alpha = 0.20 * pow(sin(Double.pi * travel), 2)
                for (c, tone) in [214.0, 165.0, 94.0].enumerated() {
                    pixels[i+c] = UInt8((Double(pixels[i+c]) * (1-alpha) + tone*alpha).rounded())
                }
            }
        }
        let image = makeImage(pixels)
        CGImageDestinationAddImage(destination, image, frameProperties)
        if frame == 28 { darkPreview = image }
        if frame == 80 { litPreview = image }
    }
}
precondition(CGImageDestinationFinalize(destination))
let verification = CGImageSourceCreateWithURL(outputURL as CFURL, nil)!
let count = CGImageSourceGetCount(verification)
let props = CGImageSourceCopyPropertiesAtIndex(verification, 0, nil)! as NSDictionary
let attrs = try FileManager.default.attributesOfItem(atPath: outputURL.path)
print("GIF saved: \(count) frames, \(width)x\(height), \(Double(frameCount)*frameDelay)s, infinite loop, \(attrs[.size]!) bytes")
precondition(count == frameCount)
precondition((props[kCGImagePropertyPixelWidth] as? Int) == width)

// One comparison sheet documents the two states without re-opening a browser.
let sheet = CGContext(data: nil, width: width * 2, height: height,
    bitsPerComponent: 8, bytesPerRow: width * 2 * 4, space: colorSpace,
    bitmapInfo: bitmapInfo)!
sheet.draw(darkPreview!, in: CGRect(x: 0, y: 0, width: width, height: height))
sheet.draw(litPreview!, in: CGRect(x: width, y: 0, width: width, height: height))
let preview = CGImageDestinationCreateWithURL(
    URL(fileURLWithPath: CommandLine.arguments[3]) as CFURL,
    UTType.png.identifier as CFString, 1, nil)!
CGImageDestinationAddImage(preview, sheet.makeImage()!, nil)
precondition(CGImageDestinationFinalize(preview))
