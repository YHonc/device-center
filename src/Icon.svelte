<script lang="ts">let {name,size=20}:{name:string;size?:number}=$props();</script>
<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
{#if name==='grid'}<rect x="3" y="3" width="7" height="7" rx="2"/><rect x="14" y="3" width="7" height="7" rx="2"/><rect x="3" y="14" width="7" height="7" rx="2"/><rect x="14" y="14" width="7" height="7" rx="2"/>
{:else if name==='mouse'}<rect x="6" y="2" width="12" height="20" rx="6"/><path d="M12 2v7M6 10h12"/>
{:else if name==='keyboard'}<rect x="2" y="6" width="20" height="13" rx="3"/><path d="M6 10h.01M10 10h.01M14 10h.01M18 10h.01M6 13h.01M10 13h.01M14 13h.01M18 13h.01M8 16h8"/>
{:else if name==='light'}<path d="M4 5h16M12 5v4M5 10h14v10H5zM8 2h8"/>
{:else if name==='refresh'}<path d="M20 8a8 8 0 1 0 0 8M20 3v5h-5"/>
{:else if name==='settings'}<path d="m9 3-1 3-3 1 1 3-2 2 2 2-1 3 3 1 1 3h6l1-3 3-1-1-3 2-2-2-2 1-3-3-1-1-3z"/><circle cx="12" cy="12" r="3"/>
{:else if name==='sun'}<circle cx="12" cy="12" r="4"/><path d="M12 2v2M12 20v2M2 12h2M20 12h2M5 5l1 1M18 18l1 1M5 19l1-1M18 6l1-1"/>
{:else if name==='moon'}<path d="M20 15A9 9 0 0 1 9 3a9 9 0 1 0 11 12Z"/>
{:else if name==='close'}<path d="m6 6 12 12M6 18 18 6"/>
{:else if name==='minimize'}<path d="M5 12h14"/>
{:else if name==='maximize'}<rect x="5" y="5" width="14" height="14" rx="2"/>
{:else if name==='arrow'}<path d="M5 12h14m-5-5 5 5-5 5"/>
{:else if name==='external'}<path d="M14 3h7v7m0-7L10 14M10 4H4v16h16v-6"/>
{:else if name==='shield'}<path d="m12 2 8 4v6c0 5-8 10-8 10S4 17 4 12V6z"/><path d="m8 12 3 3 5-6"/>
{:else if name==='power'}<path d="M12 2v10M6 5a9 9 0 1 0 12 0"/>
{:else if name==='devices'}<rect x="2" y="4" width="14" height="12" rx="2"/><path d="M6 20h6m-3-4v4"/><rect x="17" y="9" width="5" height="12" rx="1"/>
{:else if name==='wifi'}<path d="M2 8a16 16 0 0 1 20 0M5 12a11 11 0 0 1 14 0M8 16a6 6 0 0 1 8 0"/><circle cx="12" cy="20" r=".5"/>
{:else if name==='cloud'}<path d="M6 18a5 5 0 0 1-1-10 7 7 0 0 1 13 0 5 5 0 0 1 0 10Z"/><path d="M12 11v9m-3-3 3 3 3-3"/>
{:else}<circle cx="12" cy="12" r="9"/><path d="M12 8v5m0 3h.01"/>{/if}
</svg>
