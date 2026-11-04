import {OrbitControls} from 'three/addons/controls/OrbitControls.js';
import type {PerspectiveCamera} from 'three';

/** One event-driven camera controller; it never owns an animation loop. */
export function bindSceneNavigation(camera:PerspectiveCamera,canvas:HTMLCanvasElement,redraw:()=>void){
  const controls=new OrbitControls(camera,canvas);
  controls.enableDamping=false;controls.minDistance=35;controls.maxDistance=520;
  controls.rotateSpeed=.6;controls.zoomSpeed=.9;controls.panSpeed=.65;
  controls.maxTargetRadius=160;controls.listenToKeyEvents(canvas);controls.saveState();
  let touched=false,disposed=false;
  const publish=()=>{
    canvas.dataset.cameraPosition=camera.position.toArray().map(n=>n.toFixed(3)).join(',');
    canvas.dataset.cameraTarget=controls.target.toArray().map(n=>n.toFixed(3)).join(',');
    canvas.dataset.cameraDistance=controls.getDistance().toFixed(3);
  };
  const changed=()=>{if(disposed)return;touched=true;publish();redraw();};
  const reset=()=>{controls.reset();touched=false;publish();redraw();};
  const focus=()=>canvas.focus({preventScroll:true});
  const key=(event:KeyboardEvent)=>{if(event.key==='Home'){event.preventDefault();reset();}};
  controls.addEventListener('change',changed);
  canvas.addEventListener('larp-camera-reset',reset);canvas.addEventListener('pointerdown',focus);canvas.addEventListener('keydown',key);
  publish();
  return {touched:()=>touched,distance:()=>controls.getDistance(),dispose(){if(disposed)return;disposed=true;controls.removeEventListener('change',changed);controls.dispose();canvas.removeEventListener('larp-camera-reset',reset);canvas.removeEventListener('pointerdown',focus);canvas.removeEventListener('keydown',key);}};
}
