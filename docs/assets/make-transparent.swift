import AppKit

// `swift make-transparent.swift <in.png> <out.png> [--light-text]` makes the
// white background of a logo transparent: the near-white area connected to
// the image edges, with its anti-aliased rim turned into partial alpha.
// White inside the icon (its curve and dots) stays; enclosed white right of
// it, such as the holes in letters, goes. With
// `--light-text`, dark gray-blue lettering becomes light, for dark pages.

let args = CommandLine.arguments
guard args.count >= 3, let source = NSImage(contentsOfFile: args[1]),
      let cg = source.cgImage(forProposedRect: nil, context: nil, hints: nil) else {
    print("usage: swift make-transparent.swift <in.png> <out.png> [--light-text]")
    exit(1)
}
let lightText = args.contains("--light-text")
let w = cg.width, h = cg.height
var px = [UInt8](repeating: 0, count: w * h * 4)
let space = CGColorSpace(name: CGColorSpace.sRGB)!
let ctx = CGContext(data: &px, width: w, height: h, bitsPerComponent: 8, bytesPerRow: w * 4,
                    space: space, bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!
ctx.draw(cg, in: CGRect(x: 0, y: 0, width: w, height: h))

func rgb(_ i: Int) -> (Double, Double, Double) {
    (Double(px[i * 4]), Double(px[i * 4 + 1]), Double(px[i * 4 + 2]))
}

// The background: near-white pixels connected to the edges.
var background = [Bool](repeating: false, count: w * h)
var queue: [Int] = []
func nearWhite(_ i: Int) -> Bool {
    let (r, g, b) = rgb(i)
    return min(r, g, b) >= 236
}
for x in 0..<w { queue.append(x); queue.append((h - 1) * w + x) }
for y in 0..<h { queue.append(y * w); queue.append(y * w + w - 1) }
var head = 0
while head < queue.count {
    let i = queue[head]; head += 1
    if background[i] || !nearWhite(i) { continue }
    background[i] = true
    let x = i % w, y = i / w
    if x > 0 { queue.append(i - 1) }
    if x < w - 1 { queue.append(i + 1) }
    if y > 0 { queue.append(i - w) }
    if y < h - 1 { queue.append(i + w) }
}

// Enclosed white areas right of the icon are background too: the holes in
// letters such as "o" and "p". The icon is the square at the left.
var seen = background
for start in 0..<(w * h) where !seen[start] && nearWhite(start) {
    var component: [Int] = []
    var minX = w
    queue = [start]; head = 0
    seen[start] = true
    while head < queue.count {
        let i = queue[head]; head += 1
        component.append(i)
        let x = i % w, y = i / w
        minX = min(minX, x)
        for n in [x > 0 ? i - 1 : -1, x < w - 1 ? i + 1 : -1, y > 0 ? i - w : -1, y < h - 1 ? i + w : -1]
        where n >= 0 && !seen[n] && nearWhite(n) {
            seen[n] = true
            queue.append(n)
        }
    }
    if minX > h {
        for i in component { background[i] = true }
    }
}

// The rim: pixels within two of the background, blended with white.
func nearBackground(_ i: Int) -> Bool {
    let x = i % w, y = i / w
    for dy in -2...2 { for dx in -2...2 {
        let nx = x + dx, ny = y + dy
        if nx >= 0, nx < w, ny >= 0, ny < h, background[ny * w + nx] { return true }
    } }
    return false
}

var out = [UInt8](repeating: 0, count: w * h * 4)
for i in 0..<(w * h) {
    var (r, g, b) = rgb(i)
    var a = 1.0
    if background[i] {
        a = 0
    } else if nearBackground(i) {
        // Color to alpha against white.
        a = max(255 - r, 255 - g, 255 - b) / 255
        if a > 0 {
            r = 255 - (255 - r) / a
            g = 255 - (255 - g) / a
            b = 255 - (255 - b) / a
        }
    }
    if lightText && a > 0 {
        // Dark, nearly gray pixels are the lettering.
        let hi = max(r, g, b), lo = min(r, g, b)
        if hi < 120 && hi - lo < 45 {
            (r, g, b) = (230, 233, 240)
        }
    }
    out[i * 4] = UInt8((r * a).rounded())
    out[i * 4 + 1] = UInt8((g * a).rounded())
    out[i * 4 + 2] = UInt8((b * a).rounded())
    out[i * 4 + 3] = UInt8((a * 255).rounded())
}

let outCtx = CGContext(data: &out, width: w, height: h, bitsPerComponent: 8, bytesPerRow: w * 4,
                       space: space, bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!
let rep = NSBitmapImageRep(cgImage: outCtx.makeImage()!)
try! rep.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: args[2]))
