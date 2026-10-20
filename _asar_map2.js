
const fs=require('fs');
const p='D:/ai hook/deepseekharness/resources/app.asar';
const buf=fs.readFileSync(p);
const pickle= buf.readUInt32LE(4);
const jsonSize = buf.readUInt32LE(8);
const header = JSON.parse(buf.slice(12, 12+jsonSize).toString('utf8'));
const base = 8 + pickle;
const files=[];
function walk(node, prefix){
  for (const [name, val] of Object.entries(node.files||{})) {
    const full = prefix ? prefix+'/'+name : name;
    if (val.files) walk(val, full);
    else files.push({path:full, off: Number(val.offset), size: val.size});
  }
}
walk(header, '');
files.sort((a,b)=>a.off-b.off);
console.log('files', files.length, 'base', base, 'sz', buf.length);
for (const target of [82509494, 82562811]) {
  const abs = target - base;
  let hit=null;
  for (const f of files){ if (f.off<=abs && abs < f.off + f.size) { hit=f; break; } }
  console.log('offset', target, '->', JSON.stringify(hit && {path:hit.path, off:hit.off, size:hit.size}));
}
console.log('job files:', files.filter(f=>/job/i.test(f.path)).slice(0,50).map(f=>f.path).join('\n'));
