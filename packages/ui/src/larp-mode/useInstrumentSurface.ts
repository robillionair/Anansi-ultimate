import {useEffect,useRef,type RefObject} from 'react';
import {createCanvasFilmSurface} from './postprocessing/CanvasFilmSurface.ts';
import {registerDisposer} from './store.ts';

/** Event-driven instruments: no permanent animation loop or invented samples. */
export function useInstrumentSurface(ref:RefObject<HTMLCanvasElement|null>, draw:(ctx:CanvasRenderingContext2D,w:number,h:number)=>void, enabled=true) {
  const latest=useRef(draw);latest.current=draw;
  const redraw=useRef<()=>void>(()=>{});
  useEffect(()=>{
    const canvas=ref.current;if(!canvas||!enabled)return;
    const surface=createCanvasFilmSurface(canvas,{textSafe:true});if(!surface)return;
    let disposed=false,paints=0;canvas.dataset.renderer=surface.mode;
    const paint=()=>{
      if(disposed||document.hidden||!document.hasFocus())return;
      const w=canvas.clientWidth||640,h=canvas.clientHeight||480,dpr=Math.min(window.devicePixelRatio||1,2);
      surface.resize(w,h,dpr);surface.ctx.setTransform(dpr,0,0,dpr,0,0);
      latest.current(surface.ctx,w,h);surface.present();canvas.dataset.paints=String(++paints);
    };
    redraw.current=paint;
    const resize=new ResizeObserver(paint);resize.observe(canvas);
    document.addEventListener('visibilitychange',paint);window.addEventListener('focus',paint);paint();
    const cleanup=()=>{if(disposed)return;disposed=true;redraw.current=()=>{};resize.disconnect();document.removeEventListener('visibilitychange',paint);window.removeEventListener('focus',paint);surface.dispose();};
    const unregister=registerDisposer(cleanup);
    return ()=>{cleanup();unregister();};
  },[ref,enabled]);
  useEffect(()=>redraw.current());
}
