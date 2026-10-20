
const fs=require('fs');
const p='D:/ai hook/deepseekharness/resources/app.asar';
const buf=fs.readFileSync(p);
const headerSize=buf.readUInt32LE(4);
const headerBuf=buf.slice(8, 8+headerSize);
const jsonLen=headerBuf.readUInt32LE(0);
const header=JSON.parse(headerBuf.slice(4,4+jsonLen).toString('utf8'));
const base=8+headerSize;
const files=[];
function walk(node, prefix){
  for (const [name, val] of Object.entries(node.files||{})) {
    const full = prefix ? prefix+'/'+name : name;
    if (val.files) walk(val, full);
    else if (typeof val.offset === 'string' || typeof val.offset === 'number') files.push({path:full, off: Number(val.offset), size: val.size});
  }
}
walk(header, '');
files.sort((a,b)=>a.off-b.off);
const target=82509494;
let hit=null;
for (let i=0;i<files.length;i++){ if (files[i].off<=target && (i===files.length-1 || files[i+1].off>target)) { hit=files[i]; break; } }
console.log('files', files.length, 'base', base);
console.log('hit', JSON.stringify(hit));
// also find files whose name matches job registry
console.log(files.filter(f=>/job/i.test(f.path) && /.js$/.test(f.path)).slice(0,40).map(f=>f.path).join('\n'));
