function log(o){console.log(o)}function info_log(o){var n="["+(new Date).toUTCString()+"] ";console.log(`%c${n} [INFO]`,"color:blue; font-weight: bold;",o)}function error_log(o){var n="["+(new Date).toUTCString()+"] ";console.log(`%c${n} [ERR]`,"color:red; font-weight: bold;",o)}function success_log(o){var n="["+(new Date).toUTCString()+"] ";console.log(`%c${n} [SUCCESS]`,"color:green; font-weight: bold;",o)}const pSBC=(o,n,r,e)=>{let l,t,g,i,c,s,u,a=parseInt,f=Math.round,b="string"==typeof r;return"number"!=typeof o||o<-1||o>1||"string"!=typeof n||"r"!=n[0]&&"#"!=n[0]||r&&!b?null:(window.pSBCr||(window.pSBCr=(o=>{let n=o.length,r={};if(n>9){if([l,t,g,b]=o=o.split(","),(n=o.length)<3||n>4)return null;r.r=a("a"==l[3]?l.slice(5):l.slice(4)),r.g=a(t),r.b=a(g),r.a=b?parseFloat(b):-1}else{if(8==n||6==n||n<4)return null;n<6&&(o="#"+o[1]+o[1]+o[2]+o[2]+o[3]+o[3]+(n>4?o[4]+o[4]:"")),o=a(o.slice(1),16),9==n||5==n?(r.r=o>>24&255,r.g=o>>16&255,r.b=o>>8&255,r.a=f((255&o)/.255)/1e3):(r.r=o>>16,r.g=o>>8&255,r.b=255&o,r.a=-1)}return r})),u=n.length>9,u=b?r.length>9||"c"==r&&!u:u,c=window.pSBCr(n),i=o<0,s=r&&"c"!=r?window.pSBCr(r):i?{r:0,g:0,b:0,a:-1}:{r:255,g:255,b:255,a:-1},o=i?o*-1:o,i=1-o,c&&s?(e?(l=f(i*c.r+o*s.r),t=f(i*c.g+o*s.g),g=f(i*c.b+o*s.b)):(l=f((i*c.r**2+o*s.r**2)**.5),t=f((i*c.g**2+o*s.g**2)**.5),g=f((i*c.b**2+o*s.b**2)**.5)),b=c.a,s=s.a,c=b>=0||s>=0,b=c?b<0?s:s<0?b:b*i+s*o:0,u?"rgb"+(c?"a(":"(")+l+","+t+","+g+(c?","+f(1e3*b)/1e3:"")+")":"#"+(4294967296+16777216*l+65536*t+256*g+(c?f(255*b):0)).toString(16).slice(1,c?void 0:-2)):null)};export{pSBC:pSBC,log:log,info_log:info_log,error_log:error_log,success_log:success_log};