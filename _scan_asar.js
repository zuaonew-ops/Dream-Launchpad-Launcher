
const fs = require('fs');
const p = 'D:/ai hook/deepseekharness/resources/app.asar';
const buf = fs.readFileSync(p);
const hay = buf.toString('latin1');
console.log('size', buf.length);
function findAll(needle){
  const idxs=[]; let i=0;
  while(true){ const k = hay.indexOf(needle, i); if(k<0) break; idxs.push(k); i=k+1; if(idxs.length>50) break; }
  return idxs;
}
for (const n of ['unknown job', 'job_output', 'UnknownJob', 'unknownJob']) {
  const idxs = findAll(n);
  console.log('==', n, idxs.length);
  for (const k of idxs.slice(0,6)) console.log('  ctx:', JSON.stringify(hay.slice(Math.max(0,k-120), k+160)));
}
