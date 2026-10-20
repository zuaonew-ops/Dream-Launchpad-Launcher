
const fs=require('fs');
const buf=fs.readFileSync('D:/ai hook/deepseekharness/resources/app.asar');
const jsonSize = buf.readUInt32LE(8) - 4;
const header = JSON.parse(buf.slice(16, 16+jsonSize).toString('utf8'));
const base = 16 + jsonSize;
const files=[];
(function walk(node, prefix){
  for (const [name, val] of Object.entries(node.files||{})) {
    const full = prefix ? prefix+'/'+name : name;
    if (val.files) walk(val, full); else files.push({path:full, off:Number(val.offset), size: val.size});
  }
})(header, '');
files.sort((a,b)=>a.off-b.off);
console.log('files',files.length,'base',base);
for (const target of [82509494, 82562811, 18392126]) {
  const abs = target - base;
  const hit = files.find(f=>f.off<=abs && abs < f.off+f.size);
  console.log(target,'->', JSON.stringify(hit && {path:hit.path, off:hit.off, size:hit.size}));
}
const masks = /job/i;
console.log('job-ish files:\n' + files.filter(f=>masks.test(f.path) && /\.(js|cjs|mjs)$/.test(f.path) && /jobs|job-/.test(f.path)).slice(0,60).map(f=>f.path).join('\n'));
