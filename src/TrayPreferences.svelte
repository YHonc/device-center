<script lang="ts">
  import {onMount,tick} from 'svelte';
  import {command,desktop} from './api';
  let {sample=false}:{sample?:boolean}=$props();
  type Mode='logo'|'mouse'|'keyboard'|'both';
  let mode=$state<Mode>('logo');
  let selection=$state<Mode|null>(null);
  let loading=$state(false);
  let loaded=$state(false);
  let busy=$state(false);
  let error=$state('');
  let saved=$state(false);
  let disposed=false;
  const choices:{value:Mode;label:string;description:string}[]=[
    {value:'logo',label:'仅应用图标',description:'保留一个应用图标'},
    {value:'mouse',label:'鼠标电量',description:'在托盘显示鼠标电量'},
    {value:'keyboard',label:'键盘电量',description:'在托盘显示键盘电量'},
    {value:'both',label:'鼠标与键盘',description:'分别显示两个电量图标'}
  ];
  const unavailable=$derived((!desktop&&!sample)||loading||busy||(!loaded&&desktop&&!sample));
  async function load(){
    loading=true;error='';saved=false;
    try{const value=await command<{mode:Mode}>('get_tray_preferences');if(!disposed){mode=value.mode;selection=value.mode;loaded=true;}}
    catch(e){if(!disposed){loaded=false;selection=null;error=String(e);}}
    finally{if(!disposed)loading=false;}
  }
  onMount(()=>{if(desktop&&!sample)void load();else if(sample){selection=mode;loaded=true;}return()=>{disposed=true;};});
  async function save(value:Mode,trigger:HTMLInputElement){
    if(unavailable||value===mode)return;
    busy=true;error='';saved=false;
    try{const confirmed=sample?value:(await command<{mode:Mode}>('set_tray_preferences',{preferences:{mode:value}})).mode;if(!disposed){mode=confirmed;selection=confirmed;saved=true;}}
    catch(e){if(!disposed){selection=mode;error=String(e);}}
    finally{if(!disposed){busy=false;await tick();if(trigger.isConnected&&document.activeElement===document.body)trigger.focus();}}
  }
</script>

<!-- §2.1 / §3.6 / §4.1: group label first, choices second, persistent status last; existing tokens and spacing scale. -->
<section class="tray-preferences" aria-label="托盘电量显示" aria-busy={loading||busy}>
  <h3>托盘电量显示</h3>
  <p id="tray-description">选择 Windows 托盘中的显示方式。读数未知或过期时显示状态标记。</p>
  <fieldset disabled={unavailable} aria-describedby="tray-description tray-status">
    <legend class="sr-only">选择托盘显示方式</legend>
    <div class="tray-options">
      {#each choices as choice}
        <label class:selected={selection===choice.value}>
          <input type="radio" name="tray-mode" value={choice.value} bind:group={selection} onchange={event=>void save(choice.value,event.currentTarget)}/>
          <span><strong>{choice.label}</strong><small>{choice.description}</small></span>
        </label>
      {/each}
    </div>
  </fieldset>
  <div class="tray-status" id="tray-status" role="status">
    {#if loading}正在读取托盘偏好…
    {:else if busy}正在保存“{choices.find(choice=>choice.value===selection)?.label}”…
    {:else if saved}<span class="settings-success">{sample?'样本偏好已切换。':'托盘显示偏好已保存。'}</span>
    {:else if !desktop&&!sample}托盘偏好在桌面应用中可用。
    {:else if !error}选择后立即保存；可用方向键切换选项。{/if}
  </div>
  {#if error}
    <div class="tray-error"><p class="message error" role="alert">{error}</p>{#if !loaded}<button class="secondary" onclick={()=>void load()} disabled={loading}>重新读取偏好</button>{/if}</div>
  {/if}
</section>

<style>
  .tray-preferences{margin-top:24px;padding-top:24px;border-top:1px solid var(--quiet-line)}
  h3{font-size:14px;margin:0;font-weight:600}p{font-size:12px;color:var(--muted);line-height:1.6;margin:8px 0 0}
  fieldset{border:0;padding:0;margin:16px 0 0;min-width:0}
  .tray-options{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:8px}
  label{display:flex;align-items:flex-start;gap:8px;padding:12px;border:1px solid var(--quiet-line);border-radius:var(--radius-control,10px);background:var(--control);cursor:pointer;min-width:0}
  label.selected{border-color:var(--accent);background:var(--surface)}
  fieldset:disabled label{cursor:default}
  label:focus-within{outline:2px solid var(--focus-ring,var(--accent));outline-offset:2px}
  input{accent-color:var(--accent);margin:2px 0 0;flex-shrink:0}
  label>span{min-width:0}strong{font-size:12px;font-weight:600;display:block}small{font-size:12px;color:var(--muted);line-height:1.6;display:block;margin-top:4px}
  .tray-status{min-height:20px;margin-top:12px;color:var(--muted);font-size:12px;line-height:1.6}.settings-success{color:var(--success)}
  .tray-error p{margin:0 0 8px;color:var(--danger);overflow-wrap:anywhere}.tray-error button{font-size:12px}
  .sr-only{position:absolute;width:1px;height:1px;padding:0;overflow:hidden;clip:rect(0,0,0,0);white-space:nowrap;border:0}
  @media(max-width:380px){.tray-options{grid-template-columns:1fr}}
</style>
