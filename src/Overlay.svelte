<script lang="ts">
import type { Snippet } from 'svelte';
import { surfaceMotion } from './motion';
let {children,onclose,kind='modal',label}:{children:Snippet;onclose:()=>void;kind?:'modal'|'drawer';label:string}=$props();
function present(node:HTMLDialogElement){
  const active=document.activeElement instanceof HTMLElement?document.activeElement:null;
  const parent=active?.closest('dialog');
  const trigger=parent instanceof HTMLDialogElement?dialogTriggers.get(parent)??active:active;
  dialogTriggers.set(node,trigger);
  node.showModal();
  return{destroy(){
    node.close();
    dialogTriggers.delete(node);
    // A closing drawer must never steal focus from a newly opened modal.
    if(!document.querySelector('dialog[open]:not([inert])')&&trigger?.isConnected)trigger.focus({preventScroll:true});
  }};
}
</script>
<script module lang="ts">
const dialogTriggers=new WeakMap<HTMLDialogElement,HTMLElement|null>();
</script>
<dialog class="glass-overlay" use:present transition:surfaceMotion={{veil:true}} class:drawer={kind==='drawer'} aria-label={label} oncancel={e=>{e.preventDefault();onclose();}} onclick={e=>{if(e.target===e.currentTarget)onclose();}}>
  <div class="overlay-surface" transition:surfaceMotion={{drawer:kind==='drawer'}}>{@render children()}</div>
</dialog>
