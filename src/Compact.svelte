<script lang="ts">
import {onMount} from 'svelte';
import {motionRoot} from './motion';
import {listen,type UnlistenFn} from '@tauri-apps/api/event';
import {command,desktop,type Lamp} from './api';
import Icon from './Icon.svelte';
import LampPresets from './LampPresets.svelte';
import {settledLampControls} from './lampControls.js';
import {createSnapshotOrder} from './snapshotOrder.js';
import {createListenerScope} from './listenerScope.js';
import {compactCloseAction,finishCompactClose} from './windowCloseRoute.js';
import {initializeSnapshot} from './snapshotBootstrap.js';
const order=createSnapshotOrder();
type Reading={deviceId:string;model?:string;batteryPercent?:number;charging?:boolean|null;sampledAt?:number;updatedAt?:number;stale:boolean;connection?:string;error?:string};
type Light=Partial<Lamp>&{configured?:boolean;stale:boolean;error?:string;updatedAt?:number};
let readings=$state<Record<string,Reading>>({});let light=$state<Light|null>(null);let now=$state(Date.now());let error=$state('');let busy=$state(false);let brightness=$state(50);let temperature=$state(4000);let dark=$state(false);let solid=$state(false);
let pending:Partial<Lamp>={};let draining=false;let timer:ReturnType<typeof setTimeout>;
let lampQueued=$state(false);
let drainPromise:Promise<void>|null=null;let closing=$state(false);let exitAfterFlush=false;
const stale=(r:{stale:boolean;sampledAt?:number}|null|undefined)=>!r||r.stale||!r.sampledAt||now-r.sampledAt>300000;
const readingStatus=(r:Reading|undefined)=>!r?.sampledAt?(r?.connection==='disconnected'?'未连接':'未读取'):stale(r)?'已过期':r.charging?'充电中':'已响应';
const lampStatus=$derived(busy?'确认中…':!light?.sampledAt?(light?.configured?'未读取':'未连接'):stale(light)?'已过期':light.power?'已开启':'已关闭');
const confirmedLamp=$derived(light?.sampledAt&&typeof light.brightness==='number'&&typeof light.temperature==='number'&&typeof light.power==='boolean'?{brightness:light.brightness,temperature:light.temperature,power:light.power,sampledAt:light.sampledAt,stale:light.stale}:null);
const brand=(r:Reading)=>r.model?.toLowerCase().includes('kx75')?'mchose':r.model?.toLowerCase().includes('atk')?'atk':Object.entries(readings).find(([,v])=>v.deviceId===r.deviceId)?.[0];
function accept(r:Reading,source:'event'|'snapshot'='event',captured:Record<string,number>={}){const b=brand(r);if(b&&order.accept(b,r,source,captured))readings[b]={...readings[b],...r};}
function acceptLamp(r:Light,source:'event'|'snapshot'|'direct'='event',captured:Record<string,number>={}){if(!order.accept('lamp',r,source,captured))return;light={...light,...r};if((source==='direct'||!draining)&&pending.brightness===undefined&&r.brightness!==undefined)brightness=r.brightness;if((source==='direct'||!draining)&&pending.temperature===undefined&&r.temperature!==undefined)temperature=r.temperature;}
async function sync(active:()=>boolean){const captured=order.capture();try{const s=await command<{devices:Reading[];lamp:Light|null}>('get_device_snapshots');if(!active())return;s.devices.forEach(r=>accept(r,'snapshot',captured));if(s.lamp)acceptLamp(s.lamp,'snapshot',captured);}catch(e){if(active())error=String(e);}}
function drain(){if(drainPromise)return drainPromise;drainPromise=runDrain().finally(()=>drainPromise=null);return drainPromise;}
async function runDrain(){draining=true;busy=true;error='';try{while(Object.keys(pending).length){const change=pending;pending={};const r=await command<Lamp>('set_lamp',{change});acceptLamp({...r,stale:false},'direct');}}catch(e){pending={};error=String(e);if(light){brightness=light.brightness??50;temperature=light.temperature??4000;light={...light,stale:true};}}finally{draining=false;busy=false;lampQueued=Object.keys(pending).length>0;const settled=settledLampControls(light,pending);if(settled.brightness!==undefined)brightness=settled.brightness;if(settled.temperature!==undefined)temperature=settled.temperature;}}
async function closeCompact(exit=false){exitAfterFlush||=exit;if(closing)return;closing=true;clearTimeout(timer);try{await finishCompactClose(draining||Object.keys(pending).length>0,drain,()=>!!error,()=>command(exitAfterFlush?'exit_device_center':'close_device_window'));}catch(e){error=String(e);}finally{closing=false;exitAfterFlush=false;}}
function change(value:Partial<Lamp>,immediate=false){pending={...pending,...value};lampQueued=true;clearTimeout(timer);if(immediate)void drain();else timer=setTimeout(()=>void drain(),100);}
onMount(()=>{let disposed=false;const scope=createListenerScope();try{const p=JSON.parse(localStorage.getItem('glass-preferences')||'{}');dark=p.theme==='dark';solid=p.solid===true;}catch{}const interval=setInterval(()=>now=Date.now(),15000);if(desktop){void initializeSnapshot([
listen<Reading>('battery-update',({payload})=>accept(payload)).then(fn=>scope.add(fn)),
listen<Light>('lamp-update',({payload})=>acceptLamp(payload)).then(fn=>scope.add(fn)),
listen<{exit:boolean}>('request-window-close',({payload})=>{if(compactCloseAction(payload))void closeCompact();}).then(fn=>scope.add(fn))
,listen('request-compact-exit',()=>void closeCompact(true)).then(fn=>scope.add(fn))
],()=>sync(()=>!disposed),()=>!disposed).catch(e=>{if(!disposed)error=String(e);});}return()=>{disposed=true;clearInterval(interval);clearTimeout(timer);scope.close();};});
</script>
<!-- Refactoring UI §2.1 / §3.6: names and readings lead; concise real states
     follow. Flat rows replace dashboard cards; spacing follows 4/8/12/16px. -->
<div use:motionRoot class="shell compact-shell" class:dark class:solid>
<header class="compact-header" data-tauri-drag-region><strong data-tauri-drag-region>微光<span>快捷控制</span></strong><button class="icon-button" aria-label="关闭快捷面板" onclick={()=>void closeCompact()}><Icon name="close" size={14}/></button></header>
<main class="compact-body">
<section class="compact-batteries" aria-label="设备电量">
  {#each [{key:'atk',name:'ATK F1',icon:'mouse'},{key:'mchose',name:'迈从 KX75',icon:'keyboard'}] as d}
    <article class:stale={!!readings[d.key]?.sampledAt&&stale(readings[d.key])}>
      <Icon name={d.icon} size={16}/><span class="compact-device-name">{d.name}</span>
      <em>{readingStatus(readings[d.key])}</em>
      <strong title={readings[d.key]?.sampledAt?`最近读取 ${new Date(readings[d.key].sampledAt!).toLocaleTimeString('zh-CN')}`:'尚无设备读数'}>{readings[d.key]?.batteryPercent??'—'}<small>{readings[d.key]?.batteryPercent!==undefined?'%':''}</small></strong>
    </article>
  {/each}
</section>
<section class="compact-lamp" class:unavailable={stale(light)} aria-label="挂灯控制">
  <div class="compact-lamp-heading"><Icon name="light" size={16}/><strong>小米挂灯 1S</strong><span class="compact-lamp-state" aria-live="polite">{lampStatus}</span><button class="power-button" class:powered={!stale(light)&&light?.power} disabled={!desktop||stale(light)||busy||closing} aria-label="挂灯开关" aria-pressed={!stale(light)&&!!light?.power} onclick={()=>change({power:!light?.power},true)}><Icon name="power" size={15}/></button></div>
  <div class="compact-slider-row"><label for="quick-brightness">亮度</label><input style={`--range-fill:${brightness}%`} id="quick-brightness" aria-valuetext={stale(light)?(light?.sampledAt?'读数过期':'尚未读取'):`${brightness}%`} type="range" min="1" max="100" bind:value={brightness} disabled={!desktop||stale(light)||closing} oninput={()=>change({brightness})} onchange={()=>change({brightness},true)}/><output for="quick-brightness">{!stale(light)?`${brightness}%`:'—'}</output></div>
  <div class="compact-slider-row"><label for="quick-temperature">色温</label><input style={`--range-fill:${(temperature-2700)/38}%`} id="quick-temperature" aria-valuetext={stale(light)?(light?.sampledAt?'读数过期':'尚未读取'):`${temperature} K`} class="temperature" type="range" min="2700" max="6500" step="100" bind:value={temperature} disabled={!desktop||stale(light)||closing} oninput={()=>change({temperature})} onchange={()=>change({temperature},true)}/><output for="quick-temperature">{!stale(light)?`${temperature} K`:'—'}</output></div>
  <LampPresets compact confirmed={confirmedLamp} disabled={!desktop||stale(light)||closing} {busy} pending={lampQueued} onapply={change}/>
</section>
{#if error}<p class="compact-error" role="alert">{error}</p>{/if}<button class="secondary compact-open" aria-label="打开主窗查看设备设置" disabled={!desktop} onclick={()=>void command('show_main_window').catch(e=>error=String(e))}><span>设备设置</span><span>打开主窗 <Icon name="arrow" size={14}/></span></button>
</main></div>
