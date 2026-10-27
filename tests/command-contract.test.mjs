import test from'node:test';import assert from'node:assert/strict';import{readFile,readdir}from'node:fs/promises';import{fileURLToPath}from'node:url';import path from'node:path';import ts from'typescript';
const root=fileURLToPath(new URL('../',import.meta.url));
async function files(dir){const entries=await readdir(dir,{withFileTypes:true});return(await Promise.all(entries.map(e=>e.isDirectory()?files(path.join(dir,e.name)):[path.join(dir,e.name)]))).flat();}
test('所有前端实际调用的Tauri命令都注册在生产generate_handler',async()=>{
  const rust=await readFile(path.join(root,'src-tauri/src/lib.rs'),'utf8');
  const handler=rust.match(/generate_handler!\s*\[([\s\S]*?)\]/)?.[1];assert.ok(handler,'找不到生产命令注册');
  const registered=new Set(handler.split(',').map(s=>s.trim().split('::').at(-1)).filter(Boolean));
  const called=new Set(),unresolved=[];
  function names(node){if(ts.isStringLiteral(node))return[node.text];if(ts.isConditionalExpression(node))return[...names(node.whenTrue),...names(node.whenFalse)];return[];}
  for(const file of await files(path.join(root,'src'))){if(!/\.(?:ts|js|svelte)$/.test(file))continue;let code=await readFile(file,'utf8');if(file.endsWith('.svelte'))code=code.match(/<script[^>]*>([\s\S]*?)<\/script>/)?.[1]||'';const source=ts.createSourceFile(file,code,ts.ScriptTarget.Latest,true,ts.ScriptKind.TS);function visit(node){if(ts.isCallExpression(node)&&ts.isIdentifier(node.expression)&&node.expression.text==='command'){const values=node.arguments[0]?names(node.arguments[0]):[];if(!values.length)unresolved.push(path.relative(root,file)+': '+node.getText(source));values.forEach(v=>called.add(v));}ts.forEachChild(node,visit);}visit(source);}
  assert.ok(called.size>0,'未找到命令调用');assert.deepEqual(unresolved,[],'新增动态命令必须先明确其有限映射');assert.deepEqual([...called].filter(v=>!registered.has(v)),[],'前端调用了未注册的后端命令');
});
