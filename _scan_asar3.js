
const fs=require('fs');
const buf=fs.readFileSync('D:/ai hook/deepseekharness/resources/app.asar');
const hay=buf.toString('latin1');
let i=0,n=0;
while(true){const k=hay.indexOf('unknown job',i); if(k<0)break; n++; i=k+1;
  const ctx=hay.slice(Math.max(0,k-300), k+300);
  console.log('#'+n+' @'+k+':', JSON.stringify(ctx));
}
console.log('total', n);
