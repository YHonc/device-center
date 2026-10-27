<script lang="ts">
import {onMount,tick} from 'svelte';
import {emit,listen} from '@tauri-apps/api/event';
import {desktop} from './api';
import {BUILTIN_LAMP_PRESETS,LAMP_PRESET_PREFIX,createLampPresetStore,enqueueLampPreset,freshLampReading,lampPresetMatches,type LampPreset} from './lampPresets.js';
import {sameLampPreset} from './lampPresetEdit.js';
let {confirmed,disabled=false,busy=false,pending=false,compact=false,onapply,presetStore}:{confirmed:{brightness:number;temperature:number;sampledAt:number;stale?:boolean}|null;disabled?:boolean;busy?:boolean;pending?:boolean;compact?:boolean;onapply:(value:{brightness:number;temperature:number},immediate:boolean)=>void;presetStore?:ReturnType<typeof createLampPresetStore>}=$props();
let presets=$state<LampPreset[]>([]);let error=$state('');let feedback=$state('');let editing=$state<LampPreset|null>(null);let name=$state('');let now=$state(Date.now());
let editTrigger:HTMLElement|null=null;
let saveTrigger:HTMLButtonElement|undefined=$state();
let presetGroup:HTMLDivElement|undefined=$state();
let store:ReturnType<typeof createLampPresetStore>|undefined;
const ready=$derived(!disabled&&!busy&&freshLampReading(confirmed,now));
const canSave=$derived(ready&&!pending&&!editing);
const matches=(p:LampPreset)=>lampPresetMatches(p,confirmed,{now,busy,pending,disabled});
const saveHint=$derived(editing?'先保存或取消名称编辑':busy||pending?'等待灯光读回确认后可保存':!confirmed?'读取挂灯后可保存当前灯光':disabled||!freshLampReading(confirmed,now)?'刷新挂灯状态后可保存':'保存当前已确认的亮度与色温');
function apply(p:LampPreset){now=Date.now();if(!ready||!freshLampReading(confirmed,now))return;enqueueLampPreset(p,onapply);}
function beginEdit(p:LampPreset,trigger:HTMLElement|null=null){if(editing&&editing.id!==p.id&&name.trim()!==editing.name){error='请先保存或取消当前名称编辑';return;}if(editing?.id===p.id)return;editing={...p};name=p.name;editTrigger=trigger;error='';feedback='';}
function focusName(node:HTMLInputElement){queueMicrotask(()=>{if(node.isConnected){node.focus();node.select();}});}
function restoreEditFocus(trigger=editTrigger){void tick().then(()=>{if(trigger?.isConnected&&!trigger.matches(':disabled'))trigger.focus();else if(saveTrigger&&!saveTrigger.disabled)saveTrigger.focus();else presetGroup?.focus();});}
function cancelEdit(){editing=null;name='';error='';restoreEditFocus();editTrigger=null;}
function escapeEdit(event:KeyboardEvent){if(event.key==='Escape'){event.preventDefault();event.stopPropagation();cancelEdit();}}
function save(){if(!store||editing)return;now=Date.now();error='';try{const p=store.saveCurrent(confirmed,{now,busy,pending,disabled});if(p){beginEdit(p);feedback=`已保存：${p.name}，可直接修改名称`;}}catch(e){error=e instanceof Error?e.message:'保存预设失败';}}
function failedEdit(attempt:LampPreset,e:unknown,fallback:string){store?.refresh();if(sameLampPreset(presets.find(p=>p.id===attempt.id),attempt))editing=attempt;error=e instanceof Error?e.message:fallback;}
function rename(){if(!store||!editing)return;const attempt={...editing};editing=null;error='';try{store.rename(attempt.id,name,attempt);name='';feedback='预设名称已保存';restoreEditFocus();editTrigger=null;}catch(e){failedEdit(attempt,e,'重命名失败');}}
function remove(){if(!store||!editing)return;const attempt={...editing};editing=null;error='';try{store.remove(attempt.id,attempt);name='';feedback='本地预设已删除';restoreEditFocus();editTrigger=null;}catch(e){failedEdit(attempt,e,'删除预设失败');}}
onMount(()=>{
  let disposed=false;let unlisten:(()=>void)|undefined;
  store=presetStore??createLampPresetStore({storage:localStorage,notify:()=>{if(desktop)void emit('lamp-presets-changed').catch(()=>error='预设已保存；另一窗口同步通知失败，请重新打开预设');}});
  const unsubscribe=store.subscribe(s=>{presets=s.presets;error=s.error;if(editing&&!sameLampPreset(s.presets.find(p=>p.id===editing?.id),editing)){const exists=s.presets.some(p=>p.id===editing?.id);editing=null;name='';editTrigger=null;error=exists?'该预设已在另一窗口更新，请重新编辑':'该预设已在另一窗口删除，已结束编辑';}});
  const remote=(event:StorageEvent)=>{if(event.key===null||event.key.startsWith(LAMP_PRESET_PREFIX))store?.refresh();};if(!presetStore)window.addEventListener('storage',remote);
  if(desktop&&!presetStore)void listen('lamp-presets-changed',()=>store?.refresh()).then(fn=>{if(disposed)fn();else unlisten=fn;}).catch(()=>error='跨窗口预设同步不可用，请重新打开预设');
  const interval=setInterval(()=>now=Date.now(),15000);
  return()=>{disposed=true;unsubscribe();unlisten?.();clearInterval(interval);window.removeEventListener('storage',remote);};
});
</script>
<!-- §2.1 / §3.3.1 / §5.7: direct application stays primary; local editing gets visible feedback without changing device state. -->
<div class="lamp-presets" class:compact aria-label="灯光预设">
  <div class="preset-choices" bind:this={presetGroup} role="group" tabindex="-1" aria-label="点击应用灯光预设">
    {#each BUILTIN_LAMP_PRESETS as p (p.id)}<button class="preset-choice" class:matches={matches(p)} aria-label={`应用${p.name}预设，亮度 ${p.brightness}%，色温 ${p.temperature} K`} aria-pressed={matches(p)} title={`${p.name} · ${p.brightness}% · ${p.temperature} K`} disabled={!ready} onclick={()=>apply(p)}>{p.name}</button>{/each}
    {#each presets as p (p.id)}<div class="custom-preset" class:editing={editing?.id===p.id}>
      <div class="preset-chip" class:matches={matches(p)}><button class="preset-choice preset-apply" aria-label={`应用预设 ${p.name}`} aria-pressed={matches(p)} title={`${p.name} · ${p.brightness}% · ${p.temperature} K`} disabled={!ready} onclick={()=>apply(p)}>{p.name}</button><button class="preset-edit" aria-label={`编辑预设 ${p.name}`} aria-expanded={editing?.id===p.id} title={`重命名 ${p.name}`} onclick={e=>beginEdit(p,e.currentTarget)}><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" aria-hidden="true"><path d="m15 5 4 4M4 20l5-1L20 8a2.8 2.8 0 0 0-4-4L5 15z"/></svg></button></div>
      {#if editing?.id===p.id}<form class="preset-inline-editor" aria-label={`编辑 ${p.name}`} onsubmit={e=>{e.preventDefault();rename();}}>
        <div class="preset-edit-heading"><label for={`preset-name-${p.id}`}>预设名称</label><span>{p.brightness}% · {p.temperature} K</span></div><input id={`preset-name-${p.id}`} use:focusName aria-label="预设名称" aria-describedby={`preset-help-${p.id}`} bind:value={name} maxlength="32" required autocomplete="off" onkeydown={escapeEdit}/>
        <p class="preset-edit-hint" id={`preset-help-${p.id}`}>Enter 保存 · Esc 取消；只修改本地名称。</p>
        <div class="preset-edit-actions"><button class="text-button" type="submit" disabled={!name.trim()} onkeydown={escapeEdit}>保存名称</button><button class="text-button" type="button" onclick={cancelEdit} onkeydown={escapeEdit}>取消</button><button class="text-button preset-delete" type="button" onclick={remove} onkeydown={escapeEdit}>删除预设</button></div>
      </form>{/if}
    </div>{/each}
  </div>
  <button class="secondary preset-save-current" bind:this={saveTrigger} disabled={!canSave} aria-label="保存当前已确认灯光为预设" title={saveHint} onclick={save}>{compact?'保存':'保存当前'}</button>
</div>
{#if feedback}<p class="preset-feedback" class:compact role="status">{feedback}</p>{/if}
{#if error}<p class="preset-error" role="alert">{error}</p>{/if}
<style>
 .lamp-presets{display:flex;align-items:flex-start;gap:8px;margin-top:12px;min-width:0}
.preset-choices{display:flex;flex:1;flex-wrap:wrap;align-items:flex-start;gap:4px;min-width:0}
.preset-choice{flex-shrink:0;max-width:160px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;min-height:32px;padding:6px 8px;border:1px solid var(--quiet-line);border-radius:var(--radius-control,10px);background:var(--control);color:var(--ink);font-size:12px}
.preset-choice.matches{border-color:var(--accent);color:var(--accent)}
.preset-choice:focus-visible{outline-offset:-3px}
.preset-choices:focus-visible{outline:2px solid var(--focus-ring,var(--accent));outline-offset:2px;border-radius:var(--radius-control,10px)}
.preset-save-current{flex-shrink:0}
.lamp-presets.compact{margin-top:4px;gap:4px}
.compact .preset-choice{font-size:11px;min-height:28px;padding:4px 6px}
.lamp-presets.compact .secondary{min-height:28px;font-size:11px;padding:4px 8px;background:none;box-shadow:none}
.custom-preset{max-width:100%;min-width:0}.custom-preset.editing{flex-basis:100%}.preset-chip{display:inline-flex;max-width:100%;border:1px solid var(--quiet-line);border-radius:var(--radius-control,10px);background:var(--glass-panel,var(--control));overflow:hidden;vertical-align:top}.preset-chip.matches{border-color:var(--accent);color:var(--accent)}.preset-chip .preset-apply{min-width:0;border:0;border-radius:0;background:transparent;flex-shrink:1}.preset-edit{display:inline-flex;align-items:center;justify-content:center;flex:0 0 32px;width:32px;min-height:30px;padding:0;border:0;border-left:1px solid var(--quiet-line);border-radius:0;background:transparent;color:var(--muted)}.preset-edit:hover{color:var(--accent);background:var(--control)}.preset-edit:focus-visible{outline:2px solid var(--focus-ring,var(--accent));outline-offset:-3px}.preset-inline-editor{margin-top:8px;padding:12px;border:1px solid var(--glass-edge,var(--quiet-line));border-radius:var(--radius-control,10px);background:var(--glass-panel,var(--control));width:min(280px,100%);max-width:100%;box-sizing:border-box}.preset-edit-heading{display:flex;flex-wrap:wrap;align-items:baseline;justify-content:space-between;gap:8px;margin-bottom:8px;font-size:12px}.preset-edit-heading label{font-weight:600}.preset-edit-heading span{color:var(--muted);font-size:11px;font-variant-numeric:tabular-nums}.preset-inline-editor input{box-sizing:border-box;width:100%;min-width:0;font-size:12px;border:1px solid var(--quiet-line);border-radius:var(--radius-control,10px);background:var(--control);color:var(--ink);padding:8px}.preset-edit-hint{font-size:11px;line-height:1.6;color:var(--muted);margin:8px 0}.preset-edit-actions{display:flex;gap:12px;align-items:center;flex-wrap:wrap;margin-top:4px}.preset-edit-actions .text-button{font-size:12px;min-height:32px;padding:4px 0}.preset-edit-actions .preset-delete{margin-left:auto;color:var(--muted)}.preset-delete:hover{color:var(--danger)}.compact .preset-inline-editor{padding:8px}.compact .preset-edit-actions{gap:8px}.compact .preset-edit-actions .text-button{font-size:11px;min-height:28px}.compact .preset-chip .preset-apply{max-width:144px}.compact .preset-edit{flex-basis:28px;width:28px;min-height:26px}
.preset-error{color:var(--danger);font-size:12px;line-height:1.6;overflow-wrap:anywhere}.preset-feedback{color:var(--success);font-size:12px;line-height:1.6;margin:8px 0 0;overflow-wrap:anywhere}.preset-feedback.compact{font-size:11px;margin-top:4px}
</style>
