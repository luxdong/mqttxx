// Generates src-tauri/icons/icon.ico (32x32 + 16x16, uncompressed BMP entries).
// Pure Node, no dependencies.
import { mkdirSync, writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const outPath = join(dirname(fileURLToPath(import.meta.url)), '..', 'src-tauri', 'icons', 'icon.ico')

function makeImage(size) {
  const xor = Buffer.alloc(size * size * 4) // BGRA, bottom-up
  const and = Buffer.alloc(size * (size / 8)) // all zeros = opaque
  const teal = [0x7e, 0xb1, 0x06, 0xff] // #06b17e (b,g,r,a)
  const dark = [0x4e, 0x81, 0x04, 0xff] // #04814e border
  const white = [0xff, 0xff, 0xff, 0xff]
  const r = size / 5 // corner radius
  const isRounded = (x, y) => {
    // corners outside the rounded radius are transparent
    const cx = Math.min(x, size - 1 - x)
    const cy = Math.min(y, size - 1 - y)
    return cx < r && cy < r && (r - cx) * (r - cx) + (r - cy) * (r - cy) > r * r
  }
  // white "M" glyph strokes (centered zigzag), thickness scales with size
  const t = Math.max(1, Math.round(size / 10))
  const inM = (x, y) => {
    const x0 = Math.round(size * 0.22)
    const x1 = Math.round(size * 0.78)
    const yTop = Math.round(size * 0.3)
    const yBot = Math.round(size * 0.68)
    if (y < yTop || y > yBot || x < x0 - t || x > x1 + t) return false
    const w = x1 - x0
    const mid = x0 + Math.round(w / 2)
    if (x >= x0 - t && x < x0 + t) return y >= yTop && y <= yBot
    if (x > x1 - t && x <= x1 + t) return y >= yTop && y <= yBot
    if (Math.abs(x - mid) <= t && y >= yTop && y <= yTop + Math.round((yBot - yTop) * 0.7)) return true
    // diagonal strokes from outer legs to middle peak
    const dx = Math.abs(x - mid)
    if (dx <= w / 2) {
      const dy = Math.round((yBot - yTop) * (dx / (w / 2)) * 0.7)
      if (y >= yTop + dy - t && y <= yTop + dy + t) return true
    }
    return false
  }
  for (let row = 0; row < size; row++) {
    const y = size - 1 - row // bottom-up
    for (let x = 0; x < size; x++) {
      const i = (row * size + x) * 4
      if (isRounded(x, y)) continue // alpha 0
      let c = teal
      const b = Math.round(size / 16)
      if (x < b || y < b || x >= size - b || y >= size - b) c = dark
      if (inM(x, y)) c = white
      xor[i] = c[0]
      xor[i + 1] = c[1]
      xor[i + 2] = c[2]
      xor[i + 3] = c[3]
    }
  }
  // BITMAPINFOHEADER
  const header = Buffer.alloc(40)
  header.writeUInt32LE(40, 0)
  header.writeInt32LE(size, 4)
  header.writeInt32LE(size * 2, 8) // xor + and
  header.writeUInt16LE(1, 12)
  header.writeUInt16LE(32, 14)
  return Buffer.concat([header, xor, and])
}

function buildIco() {
  const sizes = [32, 16]
  const images = sizes.map(makeImage)
  const entries = []
  let offset = 6 + 16 * sizes.length
  for (let i = 0; i < sizes.length; i++) {
    const e = Buffer.alloc(16)
    e.writeUInt8(sizes[i], 0)
    e.writeUInt8(sizes[i], 1)
    e.writeUInt8(0, 2)
    e.writeUInt8(0, 3)
    e.writeUInt16LE(1, 4)
    e.writeUInt16LE(32, 6)
    e.writeUInt32LE(images[i].length, 8)
    e.writeUInt32LE(offset, 12)
    offset += images[i].length
    entries.push(e)
  }
  const dir = Buffer.alloc(6)
  dir.writeUInt16LE(0, 0)
  dir.writeUInt16LE(1, 2)
  dir.writeUInt16LE(sizes.length, 4)
  return Buffer.concat([dir, ...entries, ...images])
}

mkdirSync(dirname(outPath), { recursive: true })
writeFileSync(outPath, buildIco())
console.log('icon written:', outPath)
