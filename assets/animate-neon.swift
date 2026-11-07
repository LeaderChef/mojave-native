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
let width = 768, height = 432, frameCount = 64
let frameDelay = 1.0 / 16.0
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

for frame in 0..<frameCount {
    autoreleasepool {
        var pixels = base
        let phase = tau * Double(frame) / Double(frameCount)
        for y in 0..<height {
            let ny = Double(y) / Double(height)
            for x in 0..<width {
                let nx = Double(x) / Double(width)
                let i = (y * width + x) * 4
                let r = Double(base[i]) / 255, g = Double(base[i+1]) / 255
                let b = Double(base[i+2]) / 255
                // Only existing warm emissive pixels breathe; architecture stays fixed.
                let warm = max(0, min(1, (r - max(b, 0.3*g) - 0.16) * 3.0))
                let bright = max(0, min(1, (r - 0.40) * 2.0))
                var motion = 0.055 * sin(phase + nx * 1.8)
                    + 0.018 * sin(3*phase - nx * 4.0 + ny)
                if nx < 0.24 && ny > 0.29 && ny < 0.67 {
                    let angle = atan2((ny - 0.467) / 0.16, (nx - 0.127) / 0.08)
                    // Slow chasing highlights follow bulbs already in the roulette sign.
                    motion += 0.16 * pow(max(0, cos(angle - phase)), 8)
                }
                let gain = 1 + warm * bright * motion
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
        if frame == 16 {
            let previewURL = URL(fileURLWithPath: CommandLine.arguments[3])
            let png = CGImageDestinationCreateWithURL(previewURL as CFURL,
                UTType.png.identifier as CFString, 1, nil)!
            CGImageDestinationAddImage(png, image, nil)
            precondition(CGImageDestinationFinalize(png))
        }
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
