//! 读 PE 导入表，列出 exe 依赖的所有 DLL —— 用来判断能不能在低版本 Windows 上跑。
//!
//! 用法：node scripts/pe-imports.mjs <exe>
//! 输出：每行一个 DLL 名；带 --win7 时只列 Win7 上不存在的那些。

import { readFileSync } from "node:fs";

/** Win7 SP1 自带的系统 DLL（小写）。不在这个名单里的 apisets / 系统库，Win7 上大概率没有。 */
const WIN7_DLLS = new Set([
  "advapi32.dll", "bcrypt.dll", "cfgmgr32.dll", "comctl32.dll", "comdlg32.dll",
  "crypt32.dll", "dnsapi.dll", "dwmapi.dll", "gdi32.dll", "imm32.dll", "iphlpapi.dll",
  "kernel32.dll", "mswsock.dll", "ncrypt.dll", "netapi32.dll", "ntdll.dll",
  "ole32.dll", "oleaut32.dll", "opengl32.dll", "psapi.dll", "rpcrt4.dll",
  "secur32.dll", "setupapi.dll", "shell32.dll", "shlwapi.dll", "user32.dll",
  "userenv.dll", "usp10.dll", "uxtheme.dll", "version.dll", "winhttp.dll",
  "wininet.dll", "winmm.dll", "winspool.drv", "ws2_32.dll", "wtsapi32.dll",
  "msvcrt.dll", "ntoskrnl.exe", "powrprof.dll", "propsys.dll", "shcore.dll",
  "d3d11.dll", "dxgi.dll", "mfplat.dll", "mfuuid.dll", "wmcodecdspuuid.dll",
]);

/** Win7 没有、从 Win8 才开始有的 API 集 / 库（导入到它们 = 直接加载失败）。 */
const UCRT = [
  "api-ms-win-crt-", // 装 VC++ 2015-2022 运行库后 Win7 也有
];

const WIN8_PLUS = [
  "api-ms-win-core-synch-l1-2-0.dll", // WaitOnAddress
  "api-ms-win-core-winrt", // WinRT
  "bcryptprimitives.dll", // Win8+
  "combase.dll", // Win8+（Win7 用 ole32）
  "dcomp.dll",
  "windows.storage.dll",
];

function readImports(file) {
  const buf = readFileSync(file);
  const peOff = buf.readUInt32LE(0x3c);
  if (buf.readUInt32LE(peOff) !== 0x00004550) throw new Error("不是 PE 文件");
  const optOff = peOff + 24;
  const magic = buf.readUInt16LE(optOff);
  const pe32plus = magic === 0x20b;
  const dirOff = optOff + (pe32plus ? 112 : 96);
  const importRva = buf.readUInt32LE(dirOff + 8);

  // 节表：RVA → 文件偏移
  const numSections = buf.readUInt16LE(peOff + 6);
  const secOff = optOff + buf.readUInt16LE(peOff + 20);
  const sections = [];
  for (let i = 0; i < numSections; i++) {
    const s = secOff + i * 40;
    sections.push({
      va: buf.readUInt32LE(s + 12),
      vsize: buf.readUInt32LE(s + 8),
      raw: buf.readUInt32LE(s + 20),
      rawSize: buf.readUInt32LE(s + 16),
    });
  }
  const toFile = (rva) => {
    for (const s of sections) {
      if (rva >= s.va && rva < s.va + Math.max(s.vsize, s.rawSize)) {
        return s.raw + (rva - s.va);
      }
    }
    return null;
  };

  const out = [];
  let desc = toFile(importRva);
  if (desc === null) return out;
  for (;;) {
    const nameRva = buf.readUInt32LE(desc + 12);
    if (nameRva === 0) break;
    const nameOff = toFile(nameRva);
    if (nameOff === null) break;
    let end = nameOff;
    while (buf[end] !== 0) end++;
    out.push(buf.toString("latin1", nameOff, end));
    desc += 20;
  }
  return out;
}

const file = process.argv[2];
if (!file) {
  console.error("用法: node scripts/pe-imports.mjs <exe> [--win7]");
  process.exit(1);
}
const imports = [...new Set(readImports(file))].sort((a, b) => a.localeCompare(b));

if (process.argv.includes("--win7")) {
  const isUcrt = (lower) => UCRT.some((p) => lower.startsWith(p));
  const hard = imports.filter((d) => {
    const lower = d.toLowerCase();
    if (isUcrt(lower)) return false;
    return WIN8_PLUS.some((p) => lower.startsWith(p)) || !WIN7_DLLS.has(lower);
  });
  const ucrt = imports.filter((d) => isUcrt(d.toLowerCase()));
  console.log(`共导入 ${imports.length} 个 DLL`);
  if (ucrt.length) console.log(`  其中 ${ucrt.length} 个是 UCRT（装 VC++ 运行库就有）：${ucrt.join(", ")}`);
  console.log(`Win7 上真正缺的（硬阻断）：${hard.length} 个`);
  for (const d of hard) console.log("  " + d);
  process.exit(hard.length === 0 ? 0 : 2);
} else {
  for (const d of imports) console.log(d);
}
