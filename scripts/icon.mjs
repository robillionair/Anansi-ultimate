// Original geometric A mark, generated locally without external assets.
import fs from 'node:fs';
import zlib from 'node:zlib';
const size=64, raw=Buffer.alloc((size*4+1)*size);
for(let y=0;y<size;y++)for(let x=0;x<size;x++){
  const mark=(y>12&&y<51&&Math.abs(x-32)<(y-9)*.46&&Math.abs(x-32)>(y-9)*.46-7)||(y>36&&y<43&&x>22&&x<42);
  const i=y*(size*4+1)+1+x*4; raw.set(mark?[246,243,252,255]:[100,67,189,255],i);
}
function crc(b){let c=0xffffffff;for(const v of b){c^=v;for(let i=0;i<8;i++)c=(c>>>1)^((c&1)?0xedb88320:0);}return(c^0xffffffff)>>>0;}
function chunk(type,data){const t=Buffer.from(type),length=Buffer.alloc(4),checksum=Buffer.alloc(4);length.writeUInt32BE(data.length);checksum.writeUInt32BE(crc(Buffer.concat([t,data])));return Buffer.concat([length,t,data,checksum]);}
const ihdr=Buffer.alloc(13);ihdr.writeUInt32BE(size,0);ihdr.writeUInt32BE(size,4);ihdr[8]=8;ihdr[9]=6;
const png=Buffer.concat([Buffer.from([137,80,78,71,13,10,26,10]),chunk('IHDR',ihdr),chunk('IDAT',zlib.deflateSync(raw)),chunk('IEND',Buffer.alloc(0))]);
const ico=Buffer.alloc(22);ico.writeUInt16LE(1,2);ico.writeUInt16LE(1,4);ico[6]=size;ico[7]=size;ico.writeUInt16LE(1,10);ico.writeUInt16LE(32,12);ico.writeUInt32LE(png.length,14);ico.writeUInt32LE(22,18);
const dir='apps/desktop/src-tauri/icons';fs.mkdirSync(dir,{recursive:true});fs.writeFileSync(`${dir}/icon.png`,png);fs.writeFileSync(`${dir}/icon.ico`,Buffer.concat([ico,png]));
