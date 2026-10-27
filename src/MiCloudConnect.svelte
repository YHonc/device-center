<script lang="ts">
  import { onDestroy } from 'svelte';
  import { command, desktop, type LampConfig } from './api';
  import Icon from './Icon.svelte';
  import { selectAutomaticLamp, isCurrentAttempt } from './cloudSelection.js';
  let {onconnected,onclose,savedIp=null,expectedIp=null}:{onconnected:(config:LampConfig)=>void;onclose:()=>void;savedIp?:string|null;expectedIp?:string|null}=$props();
  type Login = {sessionId:string;qrImageDataUrl:string;expiresAt:number};
  type CloudDevice = {id:string;name:string;model:string;ip:string|null;eligible:boolean;reason:string|null};
  type DeviceList = {devices:CloudDevice[];fetchedAt:number;warnings:string[]};
  let region=$state('cn');let login=$state<Login|null>(null);let phase=$state<'idle'|'starting'|'pending'|'loading'|'ready'|'importing'|'expired'|'error'>('idle');
  let devices=$state<CloudDevice[]>([]);let error=$state('');let selection=$state('');let generation=0;let disposed=false;let timer:ReturnType<typeof setTimeout>|undefined;
  let warnings=$state<string[]>([]);let authenticated=$state(false);let fetchedAt=$state<number|null>(null);
  let selectionNote=$state('');
  const working=$derived(phase==='starting'||phase==='loading'||phase==='importing');
  const eligibleCount=$derived(devices.filter(device=>device.eligible).length);
  const selectedDevice=$derived(devices.find(device=>device.id===selection&&device.eligible));
  const statusLabel=$derived(phase==='starting'?'正在获取二维码':phase==='pending'?'等待手机确认':phase==='loading'?'正在读取设备':phase==='ready'?'选择挂灯':phase==='importing'?'正在导入配置':phase==='expired'?'二维码已过期':phase==='error'?'连接未完成':'准备连接');
  const message=(e:unknown)=>e&&typeof e==='object'&&'message'in e?String(e.message):String(e);
  async function cancel(){generation++;clearTimeout(timer);const old=login;login=null;authenticated=false;if(old)try{await command('cancel_mi_login',{sessionId:old.sessionId});}catch{}}
  export async function cleanupSession(){await cancel();}
  async function loadDevices(id:string,version:number){
    phase='loading';error='';
    try{const result=await command<DeviceList>('list_mi_devices',{sessionId:id});if(!isCurrentAttempt(version,generation,disposed))return;devices=result.devices;warnings=result.warnings||[];fetchedAt=result.fetchedAt;selection='';phase='ready';
      const match=selectAutomaticLamp(devices,savedIp,expectedIp);
      if(match.deviceId){selection=match.deviceId;selectionNote=match.reason==='saved'?'已识别此前连接的挂灯，正在导入配置。':match.reason==='expected'?'已匹配输入的设备 IP，正在导入配置。':'已识别唯一可连接的挂灯，正在导入配置。';await choose();}
      else selectionNote=match.reason==='ambiguous'?'找到多台可能的挂灯，请选择你要连接的设备。':match.reason==='missing-target'?'未找到已保存或输入 IP 对应的挂灯，请核对或手动选择。':'尚未找到可导入的挂灯，可重新读取或使用手动连接。';
    }
    catch(e){if(isCurrentAttempt(version,generation,disposed)){error=message(e);phase='error';}}
  }
  async function poll(id:string,version:number){
    if(!isCurrentAttempt(version,generation,disposed))return;
    if(login&&Date.now()>=login.expiresAt){error='二维码已过期，请重新获取后扫码。';phase='expired';return;}
    try {
      const result=await command<{status:'pending'|'authenticated';expiresAt:number}>('poll_mi_login',{sessionId:id});
      if(!isCurrentAttempt(version,generation,disposed))return;
      if(login)login={...login,expiresAt:result.expiresAt};
      if(result.status==='authenticated'){
        authenticated=true;await loadDevices(id,version);
      }else{timer=setTimeout(()=>void poll(id,version),1500);}
    }catch(e){if(isCurrentAttempt(version,generation,disposed)){error=message(e);phase=login&&Date.now()>=login.expiresAt?'expired':'error';}}
  }
  async function start(){
    if(working)return;phase='starting';error='';await cancel();if(disposed)return;const version=generation;devices=[];warnings=[];fetchedAt=null;selection='';selectionNote='';
    try{
      const value=await command<Login>('begin_mi_login',{region});
      if(!isCurrentAttempt(version,generation,disposed)){void command('cancel_mi_login',{sessionId:value.sessionId}).catch(()=>{});return;}
      if(!/^data:image\/png;base64,[A-Za-z0-9+/=]+$/.test(value.qrImageDataUrl))throw new Error('登录二维码格式不受支持。');
      login=value;phase='pending';void poll(value.sessionId,version);
    }catch(e){if(isCurrentAttempt(version,generation,disposed)){error=message(e);phase='error';}}
  }
  async function choose(){
    if(phase!=='ready'||!login||!selectedDevice)return;
    phase='importing';error='';const version=generation;
    try{const config=await command<LampConfig>('import_mi_lamp',{sessionId:login.sessionId,deviceId:selection});if(isCurrentAttempt(version,generation,disposed)){const release=cancel();const completionVersion=generation;await release;if(isCurrentAttempt(completionVersion,generation,disposed))onconnected(config);}}
    catch(e){if(isCurrentAttempt(version,generation,disposed)){error=message(e);selectionNote='导入未完成，检查错误后可重试，或选择其他设备。';phase='ready';}}
  }
  onDestroy(()=>{disposed=true;void cancel();});
</script>

<!-- §2.1 / §3.3.1 / §5.7: show the real phase first; keep one main action and wrap secondary actions on narrow windows. -->
<section class="mi-connect" aria-label="通过米家连接挂灯" aria-busy={working}>
  <div class="mi-heading"><div><h3>从米家导入挂灯</h3><p>使用米家 App 扫码登录，选择挂灯后将凭据保存在本机。</p></div><button class="text-button" onclick={onclose} disabled={phase==='importing'}>返回手动连接</button></div>
  <div class="mi-status" class:problem={phase==='error'||phase==='expired'} role="status"><Icon name={phase==='error'||phase==='expired'?'warning':phase==='ready'?'devices':'cloud'} size={16}/><span>{statusLabel}</span></div>
  {#if phase==='idle'||phase==='error'||phase==='expired'}
    {#if phase==='error'&&authenticated&&login}<div class="mi-actions"><button class="secondary" onclick={()=>{if(login)void loadDevices(login.sessionId,generation);}}>重新读取列表</button></div><p class="mi-privacy">当前扫码会话仍保留。可先重试读取设备列表；会话失效后再重新扫码。</p>{/if}
    <div class="mi-start"><label for="mi-region">米家账号地区</label><select id="mi-region" bind:value={region} disabled={!desktop}><option value="cn">中国大陆</option><option value="sg">新加坡</option><option value="de">德国</option><option value="us">美国</option><option value="tw">中国台湾</option><option value="ru">俄罗斯</option><option value="in">印度</option></select><button class="primary" disabled={!desktop} onclick={start}>{phase==='error'||phase==='expired'?'重新获取二维码':'获取登录二维码'}</button></div>
    {#if !desktop}<p class="mi-privacy">扫码连接在桌面应用中可用。</p>{/if}
    <p class="mi-privacy">登录由你发起。此流程不要求在应用中输入米家密码；不在本地持久保存账号登录凭据。仅选择设备后保存其局域网连接凭据。</p>
  {:else if phase==='starting'}<p class="mi-privacy">正在向米家请求登录二维码，请稍候…</p>
  {:else if phase==='pending'&&login}<div class="mi-qr"><img src={login.qrImageDataUrl} alt="米家登录二维码" width="184" height="184"/><div><strong>打开米家 App 扫码</strong><p>扫码后，请在手机上确认本次登录。</p><p>二维码有效期至 <time datetime={new Date(login.expiresAt).toISOString()}>{new Date(login.expiresAt).toLocaleTimeString('zh-CN')}</time></p><button class="secondary" onclick={async()=>{await cancel();phase='idle';}}>取消登录</button></div></div>
  {:else if phase==='loading'}<p class="mi-privacy">登录已确认，正在读取并识别挂灯…</p>
  {:else if phase==='ready'||phase==='importing'}
    {#if selectionNote}<p class="mi-privacy" role="status">{selectionNote}</p>{/if}
    {#if warnings.length}<div class="mi-warnings" role="status"><strong>设备列表覆盖提示</strong>{#each warnings as warning}<p>{warning}</p>{/each}</div>{/if}
    {#if !eligibleCount}<div class="mi-empty"><strong>{devices.length?'没有可导入的挂灯':'此账号下未找到设备'}</strong><p>请核对账号地区、挂灯型号与设备共享权限，也可以重新读取列表或手动连接。</p><button class="text-button" onclick={onclose}>使用手动 IP / Token</button></div>{/if}
    {#if devices.length}<fieldset class="mi-devices" disabled={phase==='importing'}><legend>选择要连接的挂灯 <span>（{eligibleCount} 台可导入）</span></legend>{#each devices as device}<label class="mi-device" class:selected={selection===device.id} class:unavailable={!device.eligible}><input type="radio" name="mi-device" value={device.id} bind:group={selection} disabled={!device.eligible}/><span><strong>{device.name}</strong><small>{device.model} · {device.ip||'未获取局域网 IP'}</small>{#if !device.eligible}<small>{device.reason||'当前设备不支持导入'}</small>{/if}</span></label>{/each}</fieldset>{/if}
    <div class="mi-actions"><button class="secondary" disabled={phase==='importing'} onclick={()=>{if(login)void loadDevices(login.sessionId,generation);}}>重新读取列表</button><button class="secondary" disabled={phase==='importing'} onclick={async()=>{await cancel();phase='idle';}}>重新登录</button>{#if eligibleCount}<button class="primary" disabled={!selectedDevice||phase==='importing'} onclick={choose}>{phase==='importing'?'正在安全保存…':'连接所选挂灯'}</button>{/if}</div>
    {#if fetchedAt}<p class="mi-privacy">列表读取于 {new Date(fetchedAt).toLocaleTimeString('zh-CN')}</p>{/if}
  {/if}
  {#if error}<p class="message error" role="alert">{error}</p>{/if}
  <p class="mi-boundary">扫码需要互联网；挂灯控制仍通过本地局域网。若账号需要其他安全验证，请在米家 App 完成后重试，或使用手动 IP / Token。</p>
</section>

<style>
  .mi-connect{border-top:1px solid var(--quiet-line);margin-top:24px;padding-top:24px;font-size:13px;min-width:0}
  .mi-heading{display:flex;align-items:flex-start;justify-content:space-between;gap:16px}.mi-heading>div{min-width:0}h3{margin:0;font-size:14px;font-weight:600}
  .mi-heading p,.mi-boundary,.mi-privacy{color:var(--muted);line-height:1.6;font-size:12px;overflow-wrap:anywhere}.mi-heading p{margin:8px 0 0}
  .mi-status{display:flex;align-items:center;gap:8px;margin:16px 0 12px;padding:12px;background:var(--control);border-radius:var(--radius-control,10px);font-size:12px;font-weight:600}.mi-status.problem{color:var(--warning);background:var(--warning-soft)}
  .mi-start{display:flex;align-items:center;gap:12px;margin-top:16px}select{background:var(--control);color:var(--ink);border:1px solid var(--line);padding:8px 12px;border-radius:var(--radius-control,10px);min-width:0;font-size:12px;min-height:36px}
  .mi-qr{display:flex;align-items:center;gap:24px;margin:24px 0}.mi-qr img{border-radius:12px;padding:12px;background:white;image-rendering:pixelated;max-width:100%;box-sizing:border-box}.mi-qr p{color:var(--muted);font-size:12px;line-height:1.6}.mi-qr time{font-variant-numeric:tabular-nums;white-space:nowrap}
  .mi-devices{border:0;padding:0;margin:16px 0;min-width:0}.mi-devices legend{font-weight:600;font-size:12px;padding:0 0 8px}.mi-devices legend span{font-weight:400;color:var(--muted)}
  .mi-device{display:flex;align-items:flex-start;gap:12px;padding:12px;border:1px solid var(--quiet-line);border-radius:var(--radius-control,10px);margin-top:8px;cursor:pointer;min-width:0}.mi-device.selected{border-color:var(--accent);background:var(--control)}.mi-device.unavailable{cursor:default}.mi-device:focus-within{outline:2px solid var(--focus-ring,var(--accent));outline-offset:2px}.mi-device>span{min-width:0;overflow-wrap:anywhere}.mi-device strong{font-size:12px}.mi-device small{display:block;color:var(--muted);margin-top:4px;font-size:12px;line-height:1.6}.mi-device input{accent-color:var(--accent);margin:2px 0 0;flex-shrink:0}
  .mi-actions{display:flex;flex-wrap:wrap;justify-content:flex-end;gap:8px;margin-top:16px}.mi-actions button{min-height:36px;font-size:12px}.mi-boundary{margin-top:24px}
  .mi-warnings{padding:12px;margin-top:16px;border:1px solid var(--warning);border-radius:var(--radius-control,10px);background:var(--warning-soft);font-size:12px;line-height:1.6;color:var(--warning)}.mi-warnings p{margin:8px 0 0;color:var(--ink);overflow-wrap:anywhere}
  .mi-empty{padding:16px;margin:16px 0;background:var(--control);border:1px dashed var(--quiet-line);border-radius:var(--radius-control,10px);font-size:12px}.mi-empty p{color:var(--muted);line-height:1.6;margin:8px 0}.message.error{color:var(--danger);overflow-wrap:anywhere}
  @media(max-width:600px){.mi-start,.mi-heading,.mi-qr{flex-direction:column;align-items:stretch}.mi-qr img{align-self:center}.mi-start{gap:8px}.mi-actions>*{flex:1 1 auto}.mi-actions .primary{flex-basis:100%}}
</style>
