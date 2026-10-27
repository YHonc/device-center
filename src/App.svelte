<script lang="ts">
  import { onMount, tick } from 'svelte';
  import {motionRoot} from './motion';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import DeviceSettings from './DeviceSettings.svelte';
  import MiCloudConnect from './MiCloudConnect.svelte';
  import Overlay from './Overlay.svelte';
  import Icon from './Icon.svelte';
  import TrayPreferences from './TrayPreferences.svelte';
  import LampPresets from './LampPresets.svelte';
  import {settledLampControls} from './lampControls.js';
  import {brandAssets} from './brandAssets';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import type { SettingsSnapshot, SettingValue } from './settings';
  import { command, desktop, type HidDevice, type Lamp, type LampConfig } from './api';
  import {createSnapshotOrder} from './snapshotOrder.js';
  import {createListenerScope} from './listenerScope.js';
  import {initializeSnapshot} from './snapshotBootstrap.js';
  import {lampNeedsCloseConfirmation,lampCanDiscardQueue} from './lampCloseGuard.js';
  const snapshotOrder=createSnapshotOrder();
  let theme = $state('light');
  let preferencesOpen=$state(false);
  let preferencesHeading:HTMLHeadingElement|undefined=$state();
  let preferencesTrigger:HTMLElement|null=null;
  async function openPreferences(){preferencesTrigger=document.activeElement instanceof HTMLElement?document.activeElement:null;preferencesOpen=true;await tick();preferencesHeading?.focus({preventScroll:true});}
  async function closePreferences(){preferencesOpen=false;await tick();preferencesTrigger?.focus({preventScroll:true});}
  let confirmWindowClose=$state(false);
  let exitRequested=$state(false);
  let cloudPanel:MiCloudConnect|undefined=$state();
  async function finishWindowClose(){if(!lampCanDiscardQueue({busy,draining})||settingsPanel?.isOperationPending())return;clearTimeout(timer);pending={};await cloudPanel?.cleanupSession();token='';configOpen=false;cloudOpen=false;settingsBrand='';confirmWindowClose=false;await command(exitRequested?'exit_device_center':'close_device_window');}
  function requestWindowClose(exit=false){exitRequested=exit;if(settingsPanel?.hasUnsavedChanges()||configOpen||lampNeedsCloseConfirmation({queued:Object.keys(pending).length,busy,draining})){confirmWindowClose=true;}else void finishWindowClose().catch(e=>notice=errorText(e));}
  async function windowAction(action:'minimize'|'maximize'|'close'){
    if(!desktop)return;
    try{const w=getCurrentWindow();if(action==='minimize')await w.minimize();else if(action==='maximize')await w.toggleMaximize();else requestWindowClose();}catch(e){notice=errorText(e);}
  }
  let solid = $state(false);
  let devices = $state<HidDevice[]>([]);
  let scanned = $state(false);
  let scanning = $state(false);
  type Battery = {batteryPercent:number;charging:boolean|null;sampledAt:number;model:string;verifiedOnHardware:boolean};
  let batteries = $state<Record<string,Battery>>({});
  let batteryErrors = $state<Record<string,string>>({});
  let batteryBusy = $state(false);
  let autostart = $state(false);
  let autostartBusy = $state(true);
  let autostartSaving = $state(false);
  let autostartKnown = $state(false);
  let autostartError = $state('');
  let settingsBrand=$state('');
  let settingsPanel:DeviceSettings|undefined=$state();
  function toggleSettings(brand:string){const current=settingsBrand;if(current){settingsPanel?.requestClose();if(settingsBrand)return;}if(current!==brand)settingsBrand=brand;}
  const responsiveBrands=[{name:'ATK F1',kind:'无线鼠标',type:'mouse',brand:'atk'},{name:'迈从 KX75',kind:'机械键盘',type:'keyboard',brand:'mchose'}];
  const loadSettings=(deviceId:string)=>command<SettingsSnapshot>('get_device_settings',{deviceId});
  const applySettings=(deviceId:string,change:Record<string,SettingValue>)=>command<SettingsSnapshot>('apply_device_settings',{deviceId,change});
  let connections = $state<Record<string,string>>({});
  const knownDeviceBrands:Record<string,string>={};
  async function toggleAutostart() {
    autostartBusy=true;autostartSaving=true;autostartError='';
    try {autostart=await command<boolean>('set_autostart',{enabled:!autostart});}
    catch(e){autostartKnown=false;autostartError=errorText(e);}
    finally{autostartBusy=false;autostartSaving=false;}
  }
  async function readAutostart(){autostartBusy=true;autostartError='';try{autostart=await command<boolean>('get_autostart');autostartKnown=true;}catch(e){autostartError=errorText(e);}finally{autostartBusy=false;}}
  const supported = (brand:string) => devices.find(d=>d.batterySupported && (brand==='atk' ? d.vendorId===0x373b && d.productId===0x101a : d.vendorId===0x3554 && d.productId===0xfa09));
  async function refreshBattery(brand:string) {
    const device=supported(brand);if(!device)return;
    batteryBusy=true;
    try {const value=await command<Battery>('read_battery',{deviceId:device.id});if(snapshotOrder.accept(brand,value,'direct')){batteries[brand]=value;batteryErrors[brand]='';connections[brand]='已响应';}}
    catch(e){batteryErrors[brand]=errorText(e);connections[brand]='连接未知';}
    finally{batteryBusy=false;}
  }
  let notice = $state('');
  let configOpen = $state(false);
  let cloudOpen = $state(false);
  let savedLampIp=$state<string|null>(null);
  function importedLamp(config:LampConfig){configured=config.configured;savedLampIp=config.ip;ip=config.ip||'';token='';lamp=null;lampError='';cloudOpen=false;configOpen=false;notice='连接已保存，正在读取挂灯状态。';void readLamp();}
  function closeConnection(){if(busy)return;configOpen=false;cloudOpen=false;token='';}
  let inventoryOpen = $state(false);
  let ip = $state('');
  let token = $state('');
  let configured = $state(false);
  let lamp = $state<Lamp|null>(null);
  let brightness = $state(50);
  let temperature = $state(4000);
  let busy = $state(false);
  let lampError = $state('');
  const lampUnavailable=$derived(!lamp||!!lampError);
  const responding=$derived(responsiveBrands.filter(d=>batteries[d.brand]&&!batteryErrors[d.brand]).length+(lamp&&!lampError?1:0));
  let pending: Partial<Lamp> = {};
  let lampQueued=$state(false);
  let draining = false;
  let timer: ReturnType<typeof setTimeout>;
  const errorText = (e:unknown) => e instanceof Error ? e.message : String(e);
  function preference() { localStorage.setItem('glass-preferences',JSON.stringify({theme,solid})); }
  function applyLamp(value:Lamp&{updatedAt?:number;stale?:boolean;error?:string},source:'direct'|'snapshot'|'event'='direct',captured:Record<string,number>={}) {if(!snapshotOrder.accept('lamp',value,source,captured))return false;lamp=value;if(!draining&&pending.brightness===undefined)brightness=value.brightness;if(!draining&&pending.temperature===undefined)temperature=value.temperature;if(source!=='direct')lampError=value.stale?(value.error||'挂灯读数已过期'):'';return true;}
  type CachedBattery=Partial<Battery>&{deviceId:string;stale:boolean;error?:string;connection:string;updatedAt?:number};
  function receiveBattery(p:CachedBattery,source:'snapshot'|'event',captured:Record<string,number>={}){
    const brand=knownDeviceBrands[p.deviceId]||['atk','mchose'].find(b=>supported(b)?.id===p.deviceId)||(p.model?.toLowerCase().includes('kx75')?'mchose':p.model?.toLowerCase().includes('atk')?'atk':null);if(!brand||!snapshotOrder.accept(brand,p,source,captured))return;
    knownDeviceBrands[p.deviceId]=brand;connections[brand]=p.connection==='connected'?'已响应':p.connection==='disconnected'?'已断开':'连接未知';batteryErrors[brand]=p.stale?(p.error||'缓存读数已过期'):'';
    if(typeof p.batteryPercent==='number'&&p.sampledAt)batteries[brand]={batteryPercent:p.batteryPercent,charging:p.charging??null,sampledAt:p.sampledAt,model:p.model||'',verifiedOnHardware:p.verifiedOnHardware===true};
  }
  async function refresh() {
    if (!desktop) { notice='浏览器预览未连接桌面后端；设备状态均为未知。'; return; }
    scanning=true; notice='';
    try { devices=await command<HidDevice[]>('list_hid_devices'); scanned=true;
      for(const brand of ['atk','mchose']) {const device=supported(brand);if(device)knownDeviceBrands[device.id]=brand;else if(batteries[brand]){batteryErrors[brand]='刷新时未发现已匹配的设备接口，历史读数已过期。';connections[brand]='已断开';}}
    }
    catch(e) { notice=errorText(e); }
    finally { scanning=false; }
  }
  async function readLamp() {
    busy=true;
    try { applyLamp(await command<Lamp>('read_lamp'));lampError=''; }
    catch(e) { lampError=errorText(e); }
    finally { busy=false; }
  }
  async function saveConfig() {
    busy=true; lampError='';
    try {
      const result=await command<LampConfig>('save_lamp_config',{ip:ip.trim(),token:token.trim()});
      configured=result.configured;savedLampIp=result.ip; token=''; configOpen=false;
      applyLamp(await command<Lamp>('read_lamp'));
    } catch(e) { lampError=errorText(e); }
    finally { busy=false; }
  }
  async function drain() {
    if (draining) return;
    draining=true; busy=true; lampError='';
    try {
      while(Object.keys(pending).length) {
        const change=pending; pending={};
        const result=await command<Lamp>('set_lamp',{change});
        if(applyLamp(result)){if(pending.brightness===undefined) brightness=result.brightness;if(pending.temperature===undefined) temperature=result.temperature;}
      }
    } catch(e) { pending={}; lampError=errorText(e); if(lamp) {brightness=lamp.brightness;temperature=lamp.temperature;} }
    finally {draining=false;busy=false;lampQueued=Object.keys(pending).length>0;const settled=settledLampControls(lamp,pending);if(settled.brightness!==undefined)brightness=settled.brightness;if(settled.temperature!==undefined)temperature=settled.temperature;}
  }
  function change(value:Partial<Lamp>, immediate=false) {
    pending={...pending,...value};lampQueued=true; clearTimeout(timer);
    if(immediate) void drain(); else timer=setTimeout(()=>void drain(),100);
  }
  async function official(brand:string) { try {await command('open_official',{brand});} catch(e){notice=errorText(e);} }
  onMount(()=>{
    let disposed=false;
    const scope=createListenerScope();
    const clock=setInterval(()=>{
      for(const brand of ['atk','mchose'])if(batteries[brand]&&Date.now()-batteries[brand].sampledAt>300000)batteryErrors[brand]||='最近读数已过期，请刷新。';
      if(lamp&&Date.now()-lamp.sampledAt>300000)lampError||='挂灯读数已过期，请重新读取。';
    },15000);
    if(desktop){
      void initializeSnapshot([
        listen<{exit:boolean}>('request-window-close',({payload})=>requestWindowClose(payload.exit)).then(fn=>scope.add(fn)),
        listen<Lamp&{stale:boolean;error?:string;updatedAt?:number}>('lamp-update',({payload:p})=>{if(p.sampledAt)applyLamp(p,'event');}).then(fn=>scope.add(fn)),
        listen<CachedBattery>('battery-update',({payload})=>receiveBattery(payload,'event')).then(fn=>scope.add(fn))
      ],async()=>{
        const captured=snapshotOrder.capture();
        const s=await command<{devices:CachedBattery[];lamp:(Lamp&{stale:boolean;error?:string;updatedAt?:number})|null}>('get_device_snapshots');
        if(disposed)return;
        for(const p of s.devices)receiveBattery(p,'snapshot',captured);
        if(s.lamp?.sampledAt)applyLamp(s.lamp,'snapshot',captured);
      },()=>!disposed).catch(e=>notice=errorText(e));
    }
    try {const saved=JSON.parse(localStorage.getItem('glass-preferences')||'{}'); theme=saved.theme==='dark'?'dark':'light';solid=saved.solid===true;}catch{}
    if(desktop) {void refresh();void command<LampConfig>('get_lamp_config').then(c=>{if(disposed)return;configured=c.configured;savedLampIp=c.ip;ip=c.ip||'';if(c.configured)void readLamp();}).catch(e=>{lampError=errorText(e);});}
    if(desktop) {
      void readAutostart();
    }
    return ()=>{disposed=true;clearTimeout(timer);clearInterval(clock);scope.close();};
  });
</script>
<svelte:window onkeydown={event=>{if(event.key==='Escape'&&preferencesOpen&&!inventoryOpen&&!configOpen&&!confirmWindowClose){event.preventDefault();void closePreferences();}}}/>
<!-- Primary: confirmed readings. Secondary: controls. Details: drawers. Existing color tokens; spacing 8/12/16/24/32; one primary action per dialog. -->
<div use:motionRoot class:dark={theme==='dark'} class:solid class="shell">
  {#if confirmWindowClose}<Overlay label="关闭设备中心" onclose={()=>confirmWindowClose=false}><section class="drawer-content"><h2>关闭窗口？</h2><p class="drawer-description">未应用的设置及尚未发送的灯光预览将被放弃，正在进行的米家扫码会话将取消。{exitRequested?'退出后后台设备服务停止。':'后台设备服务继续运行。'}</p><div class="mi-actions"><button class="secondary" onclick={()=>confirmWindowClose=false}>继续操作</button><button class="primary" disabled={busy||settingsPanel?.isOperationPending()} onclick={()=>void finishWindowClose().catch(e=>notice=errorText(e))}>{busy||settingsPanel?.isOperationPending()?'设备通信中，请稍候':'放弃并关闭'}</button></div></section></Overlay>{/if}
  <header class="titlebar" data-tauri-drag-region>
    <div class="brand" data-tauri-drag-region>{#if brandAssets.appIcon}<img src={brandAssets.appIcon} alt="" width="18" height="18"/>{:else}<span class="brand-orb"></span>{/if}<strong data-tauri-drag-region>微光</strong><span data-tauri-drag-region>设备中心</span></div>
    <span class="runtime" data-tauri-drag-region><i></i>{desktop?'本地桌面':'浏览器预览'}</span>
    <div class="window-controls"><button aria-label="最小化" disabled={!desktop} onclick={()=>windowAction('minimize')}><Icon name="minimize" size={15}/></button><button aria-label="最大化或还原" disabled={!desktop} onclick={()=>windowAction('maximize')}><Icon name="maximize" size={13}/></button><button class="window-close" aria-label="关闭到托盘" disabled={!desktop} onclick={()=>windowAction('close')}><Icon name="close" size={17}/></button></div>
  </header>
  <div class="workspace">
    <nav class="sidebar" aria-label="主导航"><div class="nav-top"><button class:nav-active={!preferencesOpen} aria-current={!preferencesOpen?'page':undefined} aria-label="设备总览" title="设备总览" onclick={()=>{inventoryOpen=false;preferencesOpen=false;}}><Icon name="grid"/></button></div><div class="nav-bottom"><button aria-label="切换明暗主题" title="切换明暗主题" onclick={()=>{theme=theme==='light'?'dark':'light';preference();}}><Icon name={theme==='light'?'moon':'sun'}/></button><button class:nav-active={preferencesOpen} aria-current={preferencesOpen?'page':undefined} aria-label="应用偏好" title="应用偏好" onclick={()=>void openPreferences()}><Icon name="settings"/></button></div></nav>
    <main class="dashboard" hidden={preferencesOpen}>
      <section class="page-heading"><div><h1>我的设备</h1><p class="page-subtitle">电量、照明与设备设置</p></div><button class="secondary refresh" disabled={scanning} onclick={refresh}><Icon name="refresh" size={15}/>{scanning?'扫描中':'刷新'}</button></section>
      {#if !desktop}<div class="preview-banner"><Icon name="info" size={16}/><span>界面预览 · 未连接桌面后端，设备状态均为未知。</span></div>{/if}
      {#if notice}<div class="notice" role="status"><span>{notice}</span><button aria-label="关闭提示" onclick={()=>notice=''}><Icon name="close" size={14}/></button></div>{/if}
      <div class="dashboard-grid">
        <section class="peripherals" aria-label="无线设备"><div class="section-label"><span>无线设备</span><span>2.4 GHz</span></div>
          {#each responsiveBrands as device}
          <article class="device-card" class:stale={!!batteryErrors[device.brand]}>
            <div class="device-card-top"><div class="model-icon"><Icon name={device.type} size={20}/></div><div class="model-name"><h2>{device.name}</h2><span>{device.kind}</span></div><button class="secondary device-settings-entry" aria-label={`${device.name}设置`} aria-expanded={settingsBrand===device.brand} onclick={()=>toggleSettings(device.brand)}><Icon name="settings" size={14}/>设置</button></div>
            <div class="peripheral-data"><div class="battery"><strong>{batteries[device.brand]?batteries[device.brand].batteryPercent:'—'}</strong><span>{batteries[device.brand]?'%':'未读取'}</span></div><div class="battery-meta"><span>{batteries[device.brand]?(batteries[device.brand].charging===null?'充电状态未知':batteries[device.brand].charging?'正在充电':'电池供电'):'连接接收器后读取'}</span>{#if batteries[device.brand]}<small>{`采样 ${new Date(batteries[device.brand].sampledAt).toLocaleTimeString('zh-CN')}`}</small>{/if}</div><span class="status" class:online={!!batteries[device.brand]&&!batteryErrors[device.brand]}><i></i>{batteries[device.brand]?(batteryErrors[device.brand]?'读数过期':'已响应'):supported(device.brand)?'待读取':'未识别'}</span></div>
            <div class="battery-track"><span style:width={`${batteries[device.brand]?.batteryPercent??0}%`}></span></div>
            {#if batteryErrors[device.brand]}<p class="compact-error" role="alert">{batteryErrors[device.brand]}</p>{/if}
            <div class="device-actions"><button class="text-button" disabled={!desktop||!supported(device.brand)||batteryBusy} onclick={()=>refreshBattery(device.brand)}><Icon name="refresh" size={13}/>{batteryBusy?'查询中':'更新电量'}</button><button class="text-button official-entry" aria-label={`${device.name}官方驱动`} disabled={!desktop} onclick={()=>official(device.brand)}>官方驱动<Icon name="external" size={13}/></button></div>
            {#if settingsBrand===device.brand}<DeviceSettings bind:this={settingsPanel} open={true} model={device.name} deviceId={desktop?(supported(device.brand)?.id??null):null} onclose={()=>settingsBrand=''} load={loadSettings} apply={applySettings}/>{/if}
          </article>{/each}
        </section>
        <section class="lighting" aria-label="桌面照明"><div class="section-label"><span>桌面照明</span><span>局域网</span></div><article class="lamp-card">
          <div class="lamp-header"><div class="model-icon"><Icon name="light" size={20}/></div><div class="model-name"><h2>小米挂灯 1S</h2><span>{configured?'连接已保存':'尚未连接'}</span></div><button class="power-button" class:powered={lamp?.power} aria-label="挂灯电源" aria-pressed={lamp?.power??false} disabled={!lamp||busy||!desktop||!!lampError} onclick={()=>change({power:!lamp?.power},true)}><Icon name="power" size={18}/></button></div>
          <div class="light-caption"><span class="status" class:online={!lampUnavailable}><i></i>{busy?'读取与确认中':lamp?(lampError?'读数过期':lamp.power?'已开启':'已关闭'):configured?'待读取':'未连接'}</span>{#if lampUnavailable}<span class="lamp-guidance">{configured?'读取状态后可调节照明':'连接挂灯后可调节照明'}</span>{/if}</div>
          <div class="sliders" class:unavailable={lampUnavailable}><label for="brightness"><span><Icon name="sun" size={15}/>亮度</span><strong>{!lampUnavailable?`${brightness}%`:'—'}</strong></label><input id="brightness" aria-valuetext={lampUnavailable?(lamp?'读数过期':'尚未读取'):`${brightness}%`} type="range" min="1" max="100" bind:value={brightness} disabled={lampUnavailable||!desktop} oninput={()=>change({brightness})} onchange={()=>change({brightness},true)}/><label for="temperature"><span><Icon name="light" size={15}/>色温</span><strong>{!lampUnavailable?`${temperature} K`:'—'}</strong></label><input class="temperature" id="temperature" aria-valuetext={lampUnavailable?(lamp?'读数过期':'尚未读取'):`${temperature} K`} type="range" min="2700" max="6500" step="100" bind:value={temperature} disabled={lampUnavailable||!desktop} oninput={()=>change({temperature})} onchange={()=>change({temperature},true)}/><div class="range-labels"><span>暖光 · 2700K</span><span>冷光 · 6500K</span></div></div>
          <p class="lamp-feedback" aria-live="polite">{busy?'等待设备确认…':lampError?'读取未成功，请核对连接后重新读取':lamp?`最近读取 ${new Date(lamp.sampledAt).toLocaleTimeString('zh-CN')}`:''}</p>
          {#if lampError}<p class="compact-error" role="alert">{lampError}</p>{/if}
          <LampPresets confirmed={lamp} disabled={!desktop||!lamp||!!lampError||confirmWindowClose} {busy} pending={lampQueued} onapply={change}/>
          <div class="lamp-actions"><button class={configured?'secondary':'primary'} onclick={()=>configOpen=true}><Icon name="cloud" size={15}/>{configured?'连接设置':'连接挂灯'}</button><button class={configured&&lampUnavailable?'primary':'text-button'} disabled={!configured||busy||!desktop} onclick={readLamp}>读取状态<Icon name="refresh" size={14}/></button></div>
        </article></section>
      </div>
      <footer class="app-footer"><span>{desktop?`${responding} / 3 台设备已响应`:'预览模式 · 控制不可用'}</span><span>微光 / 0.1</span></footer>
    </main>
  </div>
  {#if configOpen}<Overlay label="连接挂灯" onclose={closeConnection}><section class="connect-modal"><div class="modal-header"><div class="modal-symbol"><Icon name="light" size={23}/></div><button class="icon-button" aria-label="关闭连接设置" onclick={closeConnection} disabled={busy}><Icon name="close"/></button></div><h2>连接你的挂灯</h2><p class="modal-description">从米家安全导入，或直接连接局域网设备。</p><div class="connection-methods"><button aria-pressed={cloudOpen} onclick={()=>{token='';cloudOpen=true;}}><Icon name="cloud" size={16}/>米家扫码</button><button aria-pressed={!cloudOpen} onclick={()=>cloudOpen=false}><Icon name="wifi" size={16}/>手动连接</button></div>{#if cloudOpen}<MiCloudConnect bind:this={cloudPanel} onconnected={importedLamp} onclose={()=>cloudOpen=false} savedIp={savedLampIp} expectedIp={ip!==savedLampIp?ip:null}/>{:else}<form class="manual-form" onsubmit={e=>{e.preventDefault();void saveConfig();}}><label>设备 IP<input type="text" placeholder="192.168.1.100" bind:value={ip} required autocomplete="off"/></label><label>设备 Token<input type="password" placeholder="32 位十六进制字符" bind:value={token} required pattern="[a-fA-F0-9]{32}" autocomplete="off"/></label><p>Token 使用本机受保护存储，保存后不回显。</p>{#if lampError}<p class="compact-error" role="alert">{lampError}</p>{/if}<button class="primary" disabled={busy||!desktop} type="submit">{busy?'正在保存并读取…':'保存并读取设备'}<Icon name="arrow" size={16}/></button></form>{/if}</section></Overlay>{/if}
  {#if inventoryOpen}<Overlay label="设备诊断" onclose={()=>inventoryOpen=false}><section class="drawer-content"><div class="drawer-heading"><div><p class="eyebrow">连接诊断</p><h2>设备诊断</h2></div><button class="icon-button" aria-label="关闭接口" onclick={()=>inventoryOpen=false}><Icon name="close"/></button></div><p class="drawer-description">系统枚举结果。设备名称不能证明型号或协议已经适配。</p><button class="secondary" disabled={scanning} onclick={refresh}><Icon name="refresh" size={15}/>重新扫描</button>{#if !devices.length}<div class="empty-state"><Icon name="devices" size={36}/><h3>{scanned?'未发现 HID 接口':'尚未读取接口'}</h3><p>连接设备后在桌面端刷新。</p></div>{/if}{#each devices as device}<div class="interface-row"><strong>{device.product||'未命名接口'}</strong><code>VID {device.vendorId.toString(16).padStart(4,'0')} · PID {device.productId.toString(16).padStart(4,'0')}<br/>IF {device.interfaceNumber} · Usage {device.usagePage.toString(16)}:{device.usage.toString(16)}</code><span>{device.batterySupported?'已匹配电量协议':'电量协议未匹配'}</span></div>{/each}</section></Overlay>{/if}
  {#if preferencesOpen}<main class="preferences-page" aria-labelledby="preferences-title"><section class="drawer-content">
    <div class="drawer-heading"><div><h2 id="preferences-title" tabindex="-1" bind:this={preferencesHeading}>应用偏好</h2><p class="drawer-description">外观、后台运行与设备诊断</p></div><button class="icon-button" aria-label="返回设备总览" onclick={()=>void closePreferences()}><Icon name="close"/></button></div>
    <section class="preference-group" aria-labelledby="appearance-title"><h3 id="appearance-title">外观</h3><div class="preference-row"><div><strong>深色外观</strong><p>切换主窗与快捷面板的明暗主题</p></div><input aria-label="深色外观" role="switch" type="checkbox" checked={theme==='dark'} onchange={()=>{theme=theme==='dark'?'light':'dark';preference();}}/></div><div class="preference-row"><div><strong>减少透明度</strong><p>使用实色表面，提升阅读对比度</p></div><input aria-label="减少透明度" role="switch" type="checkbox" bind:checked={solid} onchange={preference}/></div><p class="preference-note">减少动态效果遵循 Windows 系统设置。</p></section>
    <section class="preference-group" aria-labelledby="background-title"><h3 id="background-title">后台运行</h3><div class="preference-row"><div><strong>开机启动</strong><p>{!desktop?'请在桌面应用中设置':autostartBusy?(autostartSaving?'正在保存启动设置…':'正在读取启动设置…'):!autostartKnown?'尚未读取启动状态':'登录 Windows 后运行设备中心'}</p></div><input aria-label="开机启动" role="switch" type="checkbox" checked={autostart} disabled={!desktop||autostartBusy||!autostartKnown} onchange={toggleAutostart}/></div>{#if autostartError}<div class="preference-error" role="alert"><p>{autostartError}</p><button class="text-button" disabled={autostartBusy} onclick={()=>void readAutostart()}>重新读取<Icon name="refresh" size={14}/></button></div>{/if}<p class="preference-note">关闭窗口后，设备服务继续在系统托盘运行。</p></section>
    <section class="preference-group" aria-label="托盘显示"><TrayPreferences/></section>
    <section class="preference-group" aria-labelledby="diagnostics-title"><h3 id="diagnostics-title">连接与诊断</h3><div class="diagnostic-entry"><div><strong>设备诊断</strong><p>设备无法识别时，检查接收器接口和协议匹配。</p></div><button class="secondary" onclick={()=>inventoryOpen=true}>查看诊断<Icon name="arrow" size={14}/></button></div></section>
  </section></main>{/if}
</div>
