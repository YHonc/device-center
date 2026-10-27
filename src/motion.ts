import { slide, type TransitionConfig } from 'svelte/transition';

const reducedMotion = () => window.matchMedia('(prefers-reduced-motion: reduce)').matches;
const smoothOut = (t: number) => 1 - Math.pow(1 - t, 4);

// Inline settings keep their card in place while the content expands vertically.
export function inlineMotion(node: HTMLElement): TransitionConfig {
  return slide(node, {
    duration: reducedMotion() || document.hidden ? 0 : 180,
    easing: smoothOut
  });
}

// One owner for every surface transform. Svelte reverses this transition in place.
export function surfaceMotion(node: HTMLElement, { drawer = false, veil = false } = {}) {
  let distance: number | undefined;
  return ({ direction = 'in' }: { direction?: 'in' | 'out' } = {}): TransitionConfig => {
    const opening = direction === 'in';
    distance ??= veil ? 0 : drawer ? (opening ? 16 : 6) : (opening ? 6 : 3);
    const style = getComputedStyle(document.documentElement);
    const token = opening ? '--duration-fast' : '--duration-quick';
    const duration = reducedMotion() || document.hidden ? 0 : parseFloat(style.getPropertyValue(token));
    return {
      duration,
      easing: smoothOut,
      css: (t, u) => `opacity:${t};${veil ? '' : `transform:translate${drawer ? 'X' : 'Y'}(${u * distance!}px);`}`
    };
  };
}

// A system preference change also finishes motion already running in this window.
export function motionRoot(node: HTMLElement) {
  const preference = window.matchMedia('(prefers-reduced-motion: reduce)');
  const finish = () => {
    if (!preference.matches && !document.hidden) return;
    for (const animation of node.getAnimations({ subtree: true })) {
      if (Number.isFinite(animation.effect?.getComputedTiming().endTime)) animation.finish();
    }
  };
  preference.addEventListener('change', finish);
  document.addEventListener('visibilitychange', finish);
  return { destroy() {
    preference.removeEventListener('change', finish);
    document.removeEventListener('visibilitychange', finish);
  } };
}
