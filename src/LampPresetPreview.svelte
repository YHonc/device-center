<script lang="ts">
import LampPresets from './LampPresets.svelte';
import {createLampPresetStore} from './lampPresets.js';
const entries=new Map<string,string>();let serial=0;
const storage={get length(){return entries.size;},key:(i:number)=>[...entries.keys()][i]??null,getItem:(key:string)=>entries.get(key)??null,setItem:(key:string,value:string)=>{entries.set(key,value);},removeItem:(key:string)=>{entries.delete(key);}};
const store=createLampPresetStore({storage,createId:()=>`fixture-${++serial}`});
const remote=createLampPresetStore({storage,notify:()=>store.refresh()});
store.save('夜间阅读与专注的超长名称用于检查截断',{brightness:65,temperature:4000});store.save('自定义 2',{brightness:30,temperature:2700});
let offline=$state(false);let calls=$state(0);let last=$state('');const compact=new URLSearchParams(location.search).get('size')==='compact';
</script>
<div class="shell" style="height:100vh;overflow:auto;padding:16px;box-sizing:border-box"><main style={`max-width:${compact?320:700}px;margin:24px auto;min-width:0`}><h2>灯光预设 · 内存样本</h2><p>不读取或写入用户 localStorage，不连接设备。</p><button onclick={()=>offline=!offline}>{offline?'模拟上线':'模拟离线'}</button><button onclick={()=>remote.rename('fixture-1','跨窗新名称')}>模拟另一窗改名</button><button onclick={()=>remote.remove('fixture-1')}>模拟另一窗删除</button><section style="margin-top:16px"><LampPresets confirmed={offline?null:{brightness:67,temperature:4809,sampledAt:Date.now()}} disabled={offline} {compact} presetStore={store} onapply={value=>{calls++;last=JSON.stringify(value);}}/></section><p role="status">应用次数：{calls} {last}</p><p>编辑和保存名称不应增加应用次数。Enter 保存，Escape 取消；离线仍可编辑和删除。</p></main></div>
