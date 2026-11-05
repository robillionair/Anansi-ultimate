/** Presentation geometry; never a source of telemetry values. */
export function instrumentBounds(w:number,h:number) {
  return {left:Math.min(56,w*.12),top:Math.min(150,h*.25),width:Math.max(1,w-Math.min(56,w*.12)-30),height:Math.max(1,h-Math.min(150,h*.25)-100)};
}
export function matrixLayout(w:number,h:number,rows:number,cols:number) {
  const b=instrumentBounds(w,h);
  const left=Math.min(130,w*.24),top=b.top+28;
  return {left,top,width:Math.max(1,w-left-28),height:Math.max(1,b.height-28),cellW:Math.max(1,w-left-28)/Math.max(cols,1),cellH:Math.max(1,b.height-28)/Math.max(rows,1)};
}
export function matrixCellAt(x:number,y:number,layout:ReturnType<typeof matrixLayout>,rows:number,cols:number) {
  if(x<layout.left||y<layout.top||x>=layout.left+layout.width||y>=layout.top+layout.height)return null;
  const col=Math.floor((x-layout.left)/layout.cellW),row=Math.floor((y-layout.top)/layout.cellH);
  return row<rows&&col<cols?{row,col}:null;
}
export function drawInstrumentStage(ctx:CanvasRenderingContext2D,w:number,h:number) {
  ctx.clearRect(0,0,w,h);ctx.fillStyle='#030305';ctx.fillRect(0,0,w,h);
  const mist=ctx.createRadialGradient(w*.6,h*.6,1,w*.6,h*.6,w*.65);
  mist.addColorStop(0,'#15100f');mist.addColorStop(.5,'#08090e');mist.addColorStop(1,'#030305');ctx.fillStyle=mist;ctx.fillRect(0,0,w,h);
  // Fixed scenic dust and a perspective calibration floor, not chart samples.
  let seed=31741;const rand=()=>{seed=(Math.imul(seed,1664525)+1013904223)>>>0;return seed/4294967296;};
  for(let i=0;i<500;i++){const x=rand()*w,y=rand()*h;ctx.fillStyle=i%7?'rgba(221,183,129,.18)':'rgba(164,187,212,.25)';ctx.fillRect(x,y,i%11? .6:1.2,i%11?.6:1.2);}
  ctx.strokeStyle='rgba(143,111,77,.09)';ctx.lineWidth=.6;
  const horizon=h*.68;
  for(let lane=-12;lane<=12;lane++){ctx.beginPath();ctx.moveTo(w*.5+lane*9,horizon);ctx.lineTo(w*.5+lane*w*.14,h);ctx.stroke();}
  for(let j=1;j<10;j++){const y=horizon+(h-horizon)*(j*j/100);ctx.beginPath();ctx.moveTo(0,y);ctx.lineTo(w,y);ctx.stroke();}
}
