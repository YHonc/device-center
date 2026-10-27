<script lang="ts">
  let {kind}:{kind:string}=$props();
</script>

<!-- Decorative silhouettes; deliberately do not imply a connected device. -->
<svg class="device-visual" viewBox="0 0 180 90" fill="none" aria-hidden="true">
  {#if kind==='mouse'}
    <ellipse cx="92" cy="77" rx="40" ry="5" fill="currentColor" opacity=".08"/>
    <path d="M65 68C55 53 58 23 74 14C83 8 102 10 111 20C122 33 128 62 115 73C102 83 75 81 65 68Z" fill="var(--control)" stroke="currentColor" stroke-opacity=".5" stroke-width="1.5"/>
    <path d="M87 12L91 40M61 42C80 47 102 45 121 37" stroke="currentColor" stroke-opacity=".25"/>
    <rect x="84" y="23" width="7" height="15" rx="3.5" fill="currentColor" opacity=".45"/>
  {:else}
    <path d="M18 27L145 20L168 66L36 76Z" fill="var(--control)" stroke="currentColor" stroke-opacity=".5" stroke-width="1.5" stroke-linejoin="round"/>
    {#each [0,1,2,3] as row}
      {#each [0,1,2,3,4,5,6,7,8,9,10] as col}
        <rect x={28+col*10.5+row*3} y={31+row*9-col*.5} width="7.5" height="6" rx="1.5" fill="currentColor" opacity={row===0?'.32':'.16'}/>
      {/each}
    {/each}
    <path d="M67 66L118 63" stroke="currentColor" stroke-width="4" stroke-linecap="round" opacity=".3"/>
  {/if}
</svg>
