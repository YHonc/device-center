import { invoke, isTauri } from '@tauri-apps/api/core';
export const desktop = isTauri();
export interface HidDevice { id:string; vendorId:number; productId:number; usagePage:number; usage:number; interfaceNumber:number; manufacturer:string|null; product:string|null; batterySupported:boolean }
export interface Lamp { power:boolean; brightness:number; temperature:number; sampledAt:number }
export interface LampConfig { configured:boolean; ip:string|null }
export async function command<T>(name:string,args?:Record<string,unknown>):Promise<T> {
  if (!desktop) throw new Error('浏览器预览未连接桌面后端，请在桌面应用中操作。');
  return invoke<T>(name,args);
}
