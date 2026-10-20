// 生成启动器图标（无第三方依赖）：PNG(32/128/256) + ICO(256)
// 用法: node scripts/gen-icon.mjs
import { writeFileSync, mkdirSync } from "node:fs";
import { deflateSync } from "node:zlib";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const outDir = join(dirname(fileURLToPath(import.meta.url)), "..", "src-tauri", "icons");

const CRC_TABLE = (() => {
  const t = new Int32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    t[n] = c;
  }
  return t;
})();

function crc32(buf) {
  let c = -1;
  for (let i = 0; i < buf.length; i++) c = CRC_TABLE[(c ^ buf[i]) & 0xff] ^ (c >>> 8);
  return (c ^ -1) >>> 0;
}

function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length);
  const body = Buffer.concat([Buffer.from(type, "ascii"), data]);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(body));
  return Buffer.concat([len, body, crc]);
}

function encodePng(size, rgba) {
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(size, 0);
  ihdr.writeUInt32BE(size, 4);
  ihdr[8] = 8; // bit depth
  ihdr[9] = 6; // RGBA
  const raw = Buffer.alloc((size * 4 + 1) * size);
  for (let y = 0; y < size; y++) {
    raw[y * (size * 4 + 1)] = 0; // filter: none
    rgba.copy(raw, y * (size * 4 + 1) + 1, y * size * 4, (y + 1) * size * 4);
  }
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk("IHDR", ihdr),
    chunk("IDAT", deflateSync(raw, { level: 9 })),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

// 画一个圆角方块 + 斜向渐变 + 中心小三角（极简"启动"意象）
function render(size) {
  const px = Buffer.alloc(size * size * 4);
  const r = size * 0.22;
  const cx = size / 2;
  const cy = size / 2;
  const inside = (x, y) => {
    const dx = Math.max(Math.abs(x - cx) - (cx - r), 0);
    const dy = Math.max(Math.abs(y - cy) - (cy - r), 0);
    return Math.hypot(dx, dy) <= r;
  };
  const tri = (x, y) => {
    // 朝右的三角形
    const w = size * 0.13;
    const h = size * 0.17;
    const x0 = cx - w * 0.9;
    const y0 = cy - h;
    const y1 = cy + h;
    if (x < x0 || x > x0 + 2 * w) return false;
    const t = (x - x0) / (2 * w);
    return y > y0 + t * (cy - y0) && y < y1 - t * (y1 - cy);
  };

  for (let y = 0; y < size; y++) {
    for (let x = 0; x < size; x++) {
      const i = (y * size + x) * 4;
      if (!inside(x + 0.5, y + 0.5)) continue;
      const t = (x / size) * 0.55 + (y / size) * 0.45;
      // #4c8dff -> #22c1a4
      let cr = Math.round(76 + (34 - 76) * t);
      let cg = Math.round(141 + (193 - 141) * t);
      let cb = Math.round(255 + (164 - 255) * t);
      if (tri(x + 0.5, y + 0.5)) {
        cr = 255;
        cg = 255;
        cb = 255;
      }
      px[i] = cr;
      px[i + 1] = cg;
      px[i + 2] = cb;
      px[i + 3] = 255;
    }
  }
  return px;
}

function encodeIco(images) {
  const header = Buffer.alloc(6);
  header.writeUInt16LE(0, 0);
  header.writeUInt16LE(1, 2);
  header.writeUInt16LE(images.length, 4);

  const dirs = [];
  const blobs = [];
  let offset = 6 + 16 * images.length;

  for (const { size, rgba } of images) {
    const rowMask = (((size + 31) >> 5) << 2); // 1bpp AND mask, 4 字节对齐
    const info = Buffer.alloc(40);
    info.writeUInt32LE(40, 0);
    info.writeInt32LE(size, 4);
    info.writeInt32LE(size * 2, 8); // XOR + AND 高度
    info.writeUInt16LE(1, 12);
    info.writeUInt16LE(32, 14);
    info.writeUInt32LE(0, 16);
    info.writeUInt32LE(size * size * 4 + rowMask * size, 20);

    const xor = Buffer.alloc(size * size * 4);
    for (let y = 0; y < size; y++) {
      for (let x = 0; x < size; x++) {
        const s = ((size - 1 - y) * size + x) * 4; // 自下而上
        const d = (y * size + x) * 4;
        xor[d] = rgba[s + 2];
        xor[d + 1] = rgba[s + 1];
        xor[d + 2] = rgba[s];
        xor[d + 3] = rgba[s + 3];
      }
    }
    const and = Buffer.alloc(rowMask * size); // 全 0 = 全部不透明
    const data = Buffer.concat([info, xor, and]);

    const dir = Buffer.alloc(16);
    dir[0] = size >= 256 ? 0 : size;
    dir[1] = size >= 256 ? 0 : size;
    dir.writeUInt16LE(1, 4);
    dir.writeUInt16LE(32, 6);
    dir.writeUInt32LE(data.length, 8);
    dir.writeUInt32LE(offset, 12);
    dirs.push(dir);
    blobs.push(data);
    offset += data.length;
  }

  return Buffer.concat([header, ...dirs, ...blobs]);
}

mkdirSync(outDir, { recursive: true });

const png32 = render(32);
const png128 = render(128);
const png256 = render(256);

writeFileSync(join(outDir, "32x32.png"), encodePng(32, png32));
writeFileSync(join(outDir, "128x128.png"), encodePng(128, png128));
writeFileSync(join(outDir, "128x128@2x.png"), encodePng(256, png256));
writeFileSync(join(outDir, "icon.png"), encodePng(256, png256));
writeFileSync(join(outDir, "icon.ico"), encodeIco([{ size: 256, rgba: png256 }]));

console.log("icons written to", outDir);
