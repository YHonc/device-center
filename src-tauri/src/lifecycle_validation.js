(async()=>{
 const phase=__PHASE__, invoke=window.__TAURI_INTERNALS__.invoke;
 const delay=ms=>new Promise(r=>setTimeout(r,ms));
 const log=message=>invoke('lifecycle_test_log',{message});
 const wait=async fn=>{for(let i=0;i<120;i++){const v=fn();if(v)return v;await delay(100);}throw Error('wait');};
 const button=text=>[...document.querySelectorAll('button')].find(b=>b.textContent.trim().includes(text));
 try {
  await wait(()=>document.querySelector('.shell,.compact-shell,main'));await log('dom_ready');
  await delay(900);
  const snap=await invoke('get_device_snapshots');await log('cache_loaded');
  if(snap.devices[0]?.batteryPercent!==55)throw Error('cache');
  if(phase!==0){await wait(()=>document.body.textContent.includes('55'));await log(phase===99?'compact_cache_restored':'main_cache_restored');return;}
  (await wait(()=>button('设备设置'))).click();await log('settings_open_clicked');
  const select=await wait(()=>document.querySelector('.settings-panel select'));await log('settings_loaded');
  select.value='500';select.dispatchEvent(new Event('change',{bubbles:true}));await delay(250);
  await invoke('plugin:window|close',{label:'main'});await log('dirty_native_close_requested');
  await wait(()=>document.body.textContent.includes('关闭窗口？'));
  (await wait(()=>button('继续操作'))).click();await log('dirty_close_cancelled_main_alive');
  (await wait(()=>button('撤回预览'))).click();
  document.querySelector('[aria-label="关闭设备设置"]').click();await delay(250);
  button('连接设置').click();await delay(250);button('米家扫码').click();await delay(250);button('获取登录二维码').click();
  await wait(()=>document.querySelector('.mi-qr'));await log('mock_cloud_pending');
  await invoke('plugin:window|close',{label:'main'});
  await wait(()=>document.body.textContent.includes('关闭窗口？'));await log('cloud_close_prompt_visible');
  button('放弃并关闭').click();
 }catch(_){await log('FAIL_frontend_phase_'+phase);}
})();
