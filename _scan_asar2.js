
const fs = require('fs');
const buf = fs.readFileSync('D:/ai hook/deepseekharness/resources/app.asar');
const hay = buf.toString('latin1');
function findAll(needle){ const o=[]; let i=0; while(true){const k=hay.indexOf(needle,i); if(k<0)break; o.push(k); i=k+1;} return o; }
for (const n of ['unknown job `','unknown job "','unknown job:','job_not_found','执行失败']) {
  const idxs = findAll(n);
  console.log('=====', n, idxs.length);
  for (const k of idxs.slice(0,8)) console.log('---', JSON.stringify(hay.slice(Math.max(0,k-160), k+200)));
}
