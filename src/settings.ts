export type SettingValue = number | string | boolean;
export interface SettingField {
  key:string; label:string; description?:string; kind:'number'|'select'|'boolean'; value:SettingValue|null;
  available:boolean; reason?:string; min?:number; max?:number; step?:number;
  options?:{label:string;value:SettingValue}[];
}
export interface SettingsSnapshot {deviceId:string;fields:SettingField[];sampledAt:number|null;stateToken?:string|null}
