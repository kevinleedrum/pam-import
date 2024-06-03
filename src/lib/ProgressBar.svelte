<script lang="ts">
  export let value: number | null = null;
  export let max = 0;

  $: percentage = value !== null ? (value / max) * 100 : 100;
</script>

<!-- Using divs because I cannot consistently style an indeterminate <progress> element -->
<div
  class="progress-bar"
  role="progressbar"
  aria-valuemin="0"
  aria-valuemax={max}
  aria-valuenow={value}
>
  <div role="presentation" style="width: {percentage}%"></div>
</div>

<style>
  .progress-bar {
    position: relative;
    margin: 0.5rem 0;
    width: 100%;
    height: 8px;
    background-color: var(--progress-bg-color);
    border-radius: 4px;
    overflow: hidden;
  }

  .progress-bar > div {
    position: absolute;
    top: 0;
    left: 0;
    height: 100%;
    background-color: var(--progress-fg-color);
    border-radius: 4px;
    transform-origin: 0% 50%;
  }

  .progress-bar:not([aria-valuenow]) > div {
    animation: indeterminate 1.5s infinite ease-in-out;
  }

  @keyframes indeterminate {
    from {
      transform: scaleX(0.5) translateX(-100%);
    }
    to {
      transform: scaleX(0.5) translateX(200%);
    }
  }
</style>
