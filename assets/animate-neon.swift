import Foundation
import CoreGraphics
import ImageIO
import UniformTypeIdentifiers

// Animate the existing emitters in original promotional art; no game data used.
guard CommandLine.arguments.count == 4 else { fatalError("usage: render INPUT.png OUTPUT.gif PREVIEW.png") }
let input = URL(fileURLWithPath: CommandLine.arguments[1])
guard let source = CGImageSourceCreateWithURL(input as CFURL, nil),
      let sourceImage = CGImageSourceCreateImageAtIndex(source, 0, nil) else { fatalError("Cannot read key art") }
let width = 768, height = 432, frameCount = 180
let frameDelay = 0.08
let space = CGColorSpaceCreateDeviceRGB()
let format = CGBitmapInfo.byteOrder32Big.rawValue | CGImageAlphaInfo.premultipliedLast.rawValue
var base = [UInt8](repeating: 0, count: width * height * 4)
base.withUnsafeMutableBytes { bytes in
    let ctx = CGContext(data: bytes.baseAddress, width: width, height: height,
        bitsPerComponent: 8, bytesPerRow: width * 4, space: space, bitmapInfo: format)!
    ctx.interpolationQuality = .high
    ctx.draw(sourceImage, in: CGRect(x: 0, y: 0, width: width, height: height))
}
func clamp(_ v: Double) -> Double { max(0, min(1, v)) }
func image(_ bytes: [UInt8]) -> CGImage {
    let provider = CGDataProvider(data: Data(bytes) as CFData)!
    return CGImage(width: width, height: height, bitsPerComponent: 8, bitsPerPixel: 32,
        bytesPerRow: width * 4, space: space, bitmapInfo: CGBitmapInfo(rawValue: format),
        provider: provider, decode: nil, shouldInterpolate: true, intent: .defaultIntent)!
}
// Ordered contours follow the actual artwork, including the raised Lucky placard.
let lucky: [(Double,Double)] = [(60,149),(70,145),(82,145),(94,146),(106,149),
    (118,153),(130,160),(141,167),(149,173),(149,189),(155,202),(159,216),
    (155,231),(146,244),(133,253),(117,258),(101,259),(85,255),(70,247),
    (58,235),(49,221),(46,206),(47,191),(52,178),(59,167),(60,149)]
let tower: [(Double,Double)] = [(149,73),(152,67),(164,60),(181,56),(204,52),
    (229,50),(254,51),(280,55),(300,61),(315,68),(320,75)]
func contour(_ x: Double, _ y: Double, _ points: [(Double,Double)]) -> (Double,Double) {
    var best = Double.infinity, phase = 0.0, travelled = 0.0
    let lengths = (0..<(points.count-1)).map { i in hypot(points[i+1].0-points[i].0, points[i+1].1-points[i].1) }
    let total = lengths.reduce(0,+)
    for i in 0..<lengths.count {
        let a=points[i], b=points[i+1], dx=b.0-a.0, dy=b.1-a.1
        let t=clamp(((x-a.0)*dx+(y-a.1)*dy)/(dx*dx+dy*dy))
        let d=hypot(x-a.0-t*dx,y-a.1-t*dy)
        if d < best { best=d; phase=(travelled+t*lengths[i])/total }
        travelled += lengths[i]
    }
    return (best, phase)
}
struct Emitter { let index: Int; let weight: Double; let phase: Double; let kind: Int }
var emitters = [Emitter]()
var counts = [0,0,0,0]
for y in 0..<height {
    for x in 0..<width {
        let i=(y*width+x)*4, px=Double(x), py=Double(y)
        let r=Double(base[i])/255, g=Double(base[i+1])/255, b=Double(base[i+2])/255
        var strength=0.0, position=0.0, kind = -1
        // Only NATIVE flickers. MOJAVE, sign border and radiation icon stay fixed.
        let top = 168.0 - 0.20 * (px-470)
        let bottom = 243.0 - 0.16 * (px-470)
        if x >= 468 && x <= 674 && py >= top-6 && py <= bottom+3 {
            strength=clamp((r-0.43)/0.22)
            kind=0
        }
        // Lucky text and 38 stay fixed: only the narrow outer bulb contour moves.
        if x >= 41 && x <= 164 && y >= 139 && y <= 264 {
            let (distance, along)=contour(px,py,lucky)
            let light=clamp((r-0.46)*5)*clamp((g-0.20)*5)
            if distance < 3.3 { strength=light*clamp((3.3-distance)/1.3); position=along; kind=1 }
        }
        // Entrance: warm bulb cores under the canopy; doors/building are excluded.
        if x >= 95 && x <= 269 && y >= 281 && y <= 314 {
            let ellipse=pow((px-182)/85,2)+pow((py-297)/17,2)
            if ellipse < 1.08 {
                strength=clamp((r-0.68)*6)*clamp((g-0.44)*6)*clamp((r-b+0.05)*4)
                position=(px-95)/174; kind=2
            }
        }
        // Tower: a travelling glow confined to the red strip beneath the top windows.
        if x >= 145 && x <= 325 && y >= 46 && y <= 80 {
            let (distance, along)=contour(px,py,tower)
            if distance < 2.6 {
                strength=clamp((2.6-distance)/1.2)*clamp((r-g-0.12)*6)*clamp((r-b-0.15)*6)
                position=along; kind=3
            }
        }
        if kind >= 0 && strength > 0.001 {
            emitters.append(Emitter(index:i,weight:strength,phase:position,kind:kind)); counts[kind] += 1
        }
    }
}
func nativePower(_ frame: Int) -> Double {
    switch frame {
    case 0..<48: return 1
    case 48..<52: return 0.12
    case 52..<58: return 0.90
    case 58..<62: return 0.04
    case 62..<68: return 0.58
    case 68..<112: return 1
    case 112..<122: return 1-Double(frame-112)/10
    case 122..<146: return 0
    case 146..<150: return 0.65
    case 150..<155: return 0.02
    case 155..<162: return 0.85
    case 162..<168: return 0.08
    default: return 1
    }
}
let output=URL(fileURLWithPath:CommandLine.arguments[2])
let dest=CGImageDestinationCreateWithURL(output as CFURL,UTType.gif.identifier as CFString,frameCount,nil)!
CGImageDestinationSetProperties(dest,[kCGImagePropertyGIFDictionary:[kCGImagePropertyGIFLoopCount:0]] as CFDictionary)
let frameProps=[kCGImagePropertyGIFDictionary:[kCGImagePropertyGIFDelayTime:frameDelay,
    kCGImagePropertyGIFUnclampedDelayTime:frameDelay]] as CFDictionary
var dark: CGImage?, lit: CGImage?
let tau=Double.pi*2
for frame in 0..<frameCount {
    autoreleasepool {
        var pixels=base
        let time=Double(frame)*frameDelay
        for emitter in emitters {
            var power=1.0
            switch emitter.kind {
            // Retain the amber glass tubes in the dim state; never black out lettering.
            case 0: power=0.40+0.60*nativePower(frame)
            case 1:
                let wave=cos(tau*(emitter.phase*3-time/2.7))
                power=0.13+0.97*pow(clamp((wave+0.25)/1.25),0.7)
            case 2:
                power=0.36+0.64*pow(0.5+0.5*sin(tau*(time/3.6-emitter.phase*0.65)),2)
            default:
                let sweep=pow(max(0,cos(tau*(emitter.phase-time/4.8))),6)
                power=0.70+0.65*sweep
            }
            let gain=1+emitter.weight*(power-1)
            for channel in 0..<3 {
                let k=emitter.index+channel
                pixels[k]=UInt8(max(0,min(255,(Double(base[k])*gain).rounded())))
            }
        }
        let frameImage=image(pixels)
        CGImageDestinationAddImage(dest,frameImage,frameProps)
        if frame == 133 { dark=frameImage }
        if frame == 24 { lit=frameImage }
    }
}
precondition(CGImageDestinationFinalize(dest))
let check=CGImageSourceCreateWithURL(output as CFURL,nil)!
precondition(CGImageSourceGetCount(check)==frameCount)
let sheet=CGContext(data:nil,width:width*2,height:height,bitsPerComponent:8,
    bytesPerRow:width*2*4,space:space,bitmapInfo:format)!
sheet.draw(dark!,in:CGRect(x:0,y:0,width:width,height:height))
sheet.draw(lit!,in:CGRect(x:width,y:0,width:width,height:height))
let preview=CGImageDestinationCreateWithURL(URL(fileURLWithPath:CommandLine.arguments[3]) as CFURL,UTType.png.identifier as CFString,1,nil)!
CGImageDestinationAddImage(preview,sheet.makeImage()!,nil); precondition(CGImageDestinationFinalize(preview))
let bytes=try FileManager.default.attributesOfItem(atPath:output.path)[.size]!
print("GIF: \(frameCount) frames, \(Double(frameCount)*frameDelay)s, \(width)x\(height), infinite loop, \(bytes) bytes")
print("Emitter pixels [Native, Lucky bulbs, canopy bulbs, tower red strip]: \(counts). All other pixels are copied unchanged.")
