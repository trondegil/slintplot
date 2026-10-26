import AppKit

// `swift make-app-icon.swift slintplot-app-icon.png` draws the app icon:
// the SlintPlot icon on the macOS icon grid: a 824 px rounded square on a
// transparent 1024 px canvas, drawn from the 128 px original's geometry.
let size = 1024.0, body = 824.0, inset = (1024.0 - 824.0) / 2
let s = body / 128.0
func p(_ x: Double, _ y: Double) -> CGPoint { CGPoint(x: inset + x * s, y: size - (inset + y * s)) }

let rep = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: 1024, pixelsHigh: 1024, bitsPerSample: 8,
    samplesPerPixel: 4, hasAlpha: true, isPlanar: false, colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: rep)
let cg = NSGraphicsContext.current!.cgContext

// Soft shadow under the tile, as macOS icons have.
let tile = CGPath(roundedRect: CGRect(x: inset, y: inset, width: body, height: body),
                  cornerWidth: body * 0.225, cornerHeight: body * 0.225, transform: nil)
cg.saveGState()
cg.setShadow(offset: CGSize(width: 0, height: -10), blur: 24, color: NSColor.black.withAlphaComponent(0.3).cgColor)
cg.addPath(tile); cg.setFillColor(NSColor.black.cgColor); cg.fillPath()
cg.restoreGState()

// Indigo to teal, top left to bottom right.
cg.saveGState()
cg.addPath(tile); cg.clip()
let gradient = CGGradient(colorsSpace: CGColorSpaceCreateDeviceRGB(), colors: [
    NSColor(srgbRed: 0x31/255, green: 0x32/255, blue: 0xad/255, alpha: 1).cgColor,
    NSColor(srgbRed: 0x0f/255, green: 0xbe/255, blue: 0xb3/255, alpha: 1).cgColor] as CFArray, locations: [0, 1])!
cg.drawLinearGradient(gradient, start: p(0, 0), end: p(128, 128), options: [])

// A faint grid.
cg.setStrokeColor(NSColor.white.withAlphaComponent(0.22).cgColor)
cg.setLineWidth(1.6 * s)
for v in [42.0, 85.0] {
    cg.move(to: p(v, 0)); cg.addLine(to: p(v, 128))
    cg.move(to: p(0, v)); cg.addLine(to: p(128, v))
}
cg.strokePath()

// The curve, and its three points.
cg.setStrokeColor(NSColor.white.cgColor)
cg.setLineWidth(4.2 * s)
cg.setLineCap(.round)
cg.move(to: p(20, 104))
cg.addCurve(to: p(64, 62), control1: p(50, 104), control2: p(64, 90))
cg.addCurve(to: p(108, 20), control1: p(64, 34), control2: p(78, 20))
cg.strokePath()
cg.setFillColor(NSColor.white.cgColor)
for c in [p(20, 104), p(64, 62), p(108, 20)] {
    cg.fillEllipse(in: CGRect(x: c.x - 5.5 * s, y: c.y - 5.5 * s, width: 11 * s, height: 11 * s))
}
cg.restoreGState()

NSGraphicsContext.current = nil
try! rep.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: CommandLine.arguments[1]))
