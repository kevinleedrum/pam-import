<script lang="ts">
  import { invoke } from "@tauri-apps/api/tauri";
  import { onMount } from "svelte";
  import { open } from "@tauri-apps/api/dialog";
  import { pictureDir } from "@tauri-apps/api/path";
  import {
    listen,
    type Event as TauriCustomEvent,
  } from "@tauri-apps/api/event";
  import { type as getOsType } from "@tauri-apps/api/os";

  import ProgressBar from "../lib/ProgressBar.svelte";

  const WINDOWS_OS_TYPE = "Windows_NT";

  let sourceDir = "";
  let destinationDir = "";
  let status = "";
  let template = "%Y-%m/%Y%m%d-%H%M%S";
  let example = "";
  let progressValue: number | null = 0;
  let progressMax = 0;
  let isStarted = false;
  let isDone = false;
  let osType = "";
  let formStatus = "";
  let logs: string[] = [];
  let logDebounce = 0;

  let unlistenProgress: () => void;
  let unlistenLog: () => void;

  onMount(async () => {
    sourceDir = await invoke("get_default_source");
    destinationDir = await pictureDir();
    osType = await getOsType();
    if (osType === WINDOWS_OS_TYPE) template = template.replaceAll("/", "\\");
    await getTemplateExample();
  });

  async function getTemplateExample() {
    example = await invoke("get_filename_template_example", {
      filenameTemplate: template,
    });
  }

  async function chooseDir(forDestionation = false) {
    const selected = await open({
      defaultPath: forDestionation ? destinationDir : sourceDir,
      directory: true,
      multiple: false,
    });
    const dir = Array.isArray(selected) ? selected[0] : selected;
    if (!dir) return;
    if (forDestionation) destinationDir = dir;
    else sourceDir = dir;
  }

  async function chooseSourceDir() {
    await chooseDir();
  }

  async function chooseDestinationDir() {
    await chooseDir(true);
  }

  async function start() {
    if (isStarted || !validateStart()) return;
    isStarted = true;
    unlistenProgress = await listen("progress", onProgress);
    unlistenLog = await listen("log", onLog);
    const [importedCount, skippedCount]: number[] = await invoke("start", {
      source: sourceDir,
      destination: destinationDir,
      filenameTemplate: template.replaceAll("\\", "/"),
    });
    if (progressValue === null) progressValue = 0;
    isDone = true;
    status = `Imported ${importedCount} files`;
    if (skippedCount) status += `, skipped ${skippedCount} existing files`;
  }

  function onProgress(
    e: TauriCustomEvent<{
      status: string;
      progress_value: number;
      progress_max: number;
    }>
  ) {
    if (e.payload.status) status = e.payload.status;
    if (osType === WINDOWS_OS_TYPE) status = status.replaceAll("/", "\\");
    progressValue = e.payload.progress_value;
    progressMax = e.payload.progress_max;
  }

  function onLog(e: TauriCustomEvent<{ message: string }>) {
    let message = e.payload.message;
    if (osType === WINDOWS_OS_TYPE) message = message.replaceAll("/", "\\");
    logs.push(message);
    refreshLogSection();
  }

  function refreshLogSection() {
    if (logDebounce) clearTimeout(logDebounce);
    logDebounce = setTimeout(() => {
      logs = logs;
    }, 100);
  }

  async function stop() {
    if (isDone) return reset();
    await invoke("stop");
  }

  function reset() {
    if (unlistenProgress) unlistenProgress();
    if (unlistenLog) unlistenLog();
    status = "";
    progressValue = 0;
    logs = [];
    isStarted = false;
    isDone = false;
  }

  function validateStart() {
    if (!sourceDir || !destinationDir) {
      formStatus = "Source and destination directories are required.";
      return false;
    }
    if (sourceDir === destinationDir) {
      formStatus = "Source and destination directories cannot be the same.";
      return false;
    }
    if (!template || !example) {
      formStatus = "A valid filename template is required.";
      return false;
    }
    formStatus = "";
    return true;
  }
</script>

<div class="container">
  {#if !isStarted}
    <form on:submit|preventDefault={start}>
      <label for="source">Source</label>
      <div class="dir-field">
        <button type="button" on:click={chooseSourceDir}>Select&hellip;</button>
        <input id="source" bind:value={sourceDir} disabled />
      </div>

      <label for="destination">Destination</label>
      <div class="dir-field">
        <button type="button" on:click={chooseDestinationDir}>
          Select&hellip;
        </button>
        <input id="destination" bind:value={destinationDir} disabled />
      </div>

      <label for="template">Filename Template</label>
      <div class="example-field">
        <input
          id="template"
          bind:value={template}
          on:input={getTemplateExample}
        />
        <p>Ex: <code>{example || "Invalid format"}</code></p>
      </div>

      <button type="submit">Import</button>

      <p class="form-status">{formStatus}</p>
    </form>
  {:else}
    <h1>Importing {sourceDir} to {destinationDir}</h1>
    <p class="status">{status}</p>

    <ProgressBar max={progressMax} value={progressValue} />

    {#if progressValue === null}
      Found {progressMax} files
    {:else}
      <p>
        {#if !isDone}
          Importing {progressValue} of {progressMax} files
        {:else}
          &nbsp;
        {/if}
      </p>
    {/if}

    <button
      class="stop-button"
      type="button"
      on:click={stop}
      disabled={!isStarted}
    >
      {isDone ? "Done" : "Stop"}
    </button>

    <section class="log">
      {#each logs as log}
        <p>{log}</p>
      {/each}
      <div id="scroll-anchor"></div>
    </section>
  {/if}
</div>

<style>
  :global(html, body) {
    height: 100%;
  }
  :global(body) {
    display: flex;
    flex-direction: column;
  }
  :root {
    --bg-color: #f0f0f0;
    --text-color: #0f0f0f;
    --button-color: #fff;
    --primary-color: #c53b18;
    --secondary-color: #fec2b9;
    --primary-button-color: var(--primary-color);
    --primary-button-text-color: #fff;
    --input-color: #f4f4f4;
    --focus-color: #000;
    --progress-bg-color: #fff;
    --progress-fg-color: var(--primary-color);
    --border-color: rgba(0, 0, 0, 0.2);
    --shadow-color: rgba(0, 0, 0, 0.05);
    --form-status-color: var(--secondary-color);
    font-family: "Roboto", system-ui, sans-serif;
    font-size: 14px;
    line-height: 24px;
    font-weight: 400;

    color: var(--text-color);
    background-color: var(--bg-color);

    font-synthesis: none;
    text-rendering: optimizeLegibility;
    -webkit-font-smoothing: antialiased;
    -moz-osx-font-smoothing: grayscale;
    -webkit-text-size-adjust: 100%;
  }

  h1 {
    font-size: 1rem;
    font-weight: 700;
  }

  .container {
    padding: 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    height: calc(100% - 2rem);
  }

  form {
    display: contents;
  }

  label,
  h1 {
    margin: 0;
    justify-self: start;
    font-weight: 700;
    letter-spacing: 0.02rem;
    user-select: none;
    -webkit-user-select: none;
  }

  label:not(:first-of-type) {
    margin-top: 1rem;
  }

  .form-status {
    position: fixed;
    bottom: 0;
    left: 0;
    width: 100%;
    height: 2rem;
    font-size: 0.9rem;
    display: flex;
    align-items: center;
    justify-content: center;
    background-color: var(--form-status-color);
    color: var(--form-status-text-color);
  }

  .dir-field {
    display: flex;
    gap: 0.25rem;
    align-items: stretch;
  }

  .dir-field input {
    flex: 1;
  }

  .example-field {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    align-items: stretch;
  }

  p {
    margin: 0;
  }

  .example-field p {
    margin-top: 0.25rem;
    font-size: 0.9rem;
    padding-left: 0.75rem;
  }

  input,
  button {
    color: inherit;
    border-radius: 4px;
    padding: 0.5rem 0.75rem;
    font-size: 1rem;
    font-family: inherit;
    border: 1px solid var(--border-color);
    transition: border-color 0.1s;
  }

  input {
    background-color: var(--input-color);
  }

  input:disabled,
  input:read-only {
    background-color: transparent;
    border-color: transparent;
  }

  button {
    cursor: pointer;
    font-weight: 500;
    background-color: var(--button-color);
    user-select: none;
    -webkit-user-select: none;
  }

  button:hover:not(:active) {
    transform: translateY(-1px);
    box-shadow: 0 2px 0 var(--shadow-color);
  }

  input:focus,
  button:focus {
    border-color: var(--focus-color);
  }

  input,
  button {
    outline: none;
  }

  button[type="submit"] {
    border-color: transparent;
    background-color: var(--primary-button-color);
    color: var(--primary-button-text-color);
    font-weight: 700;
  }

  button[type="submit"],
  button.stop-button {
    margin-top: 2rem;
    padding: 0.75rem 3rem;
    height: 3rem;
    align-self: center;
  }

  .status {
    font-size: 0.9rem;
    margin-top: 2rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  section.log {
    flex: 1;
    overflow: auto;
    margin-top: 2rem;
    padding: 0.5rem;
    border: 1px solid var(--border-color);
    border-radius: 4px;
    white-space: nowrap;
  }

  section.log p {
    margin: 0;
    font-size: 0.9rem;
  }

  @media (prefers-color-scheme: dark) {
    :root {
      --secondary-color: #341b14;
      --bg-color: #111;
      --text-color: #f0f0f0;
      --button-color: #222;
      --input-color: #2a2a2a;
      --focus-color: #fff;
      --progress-bg-color: #222;
      --border-color: rgba(192, 192, 192, 0.2);
      --shadow-color: rgba(192, 192, 192, 0.05);
      --form-status-text-color: #fff;
    }
  }
</style>
