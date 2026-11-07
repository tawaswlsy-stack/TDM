<script lang="ts">
  import { onMount, tick } from "svelte";
  import { save } from "@tauri-apps/plugin-dialog";
  import type { ActivityEntry } from "../bindings/ActivityEntry";
  import { locale } from "../i18n.svelte";
  import { api } from "../api";
  import Icon from "./Icon.svelte";

  let entries = $state<ActivityEntry[]>([]);
  let level = $state("");
  let engine = $state("");
  let operation = $state("");
  let order = $state<"newest" | "oldest">("newest");
  let autoScroll = $state(true);
  let list: HTMLDivElement | undefined = $state();
  let problem = $state<string | null>(null);
  let message = $state<string | null>(null);

  const engines = $derived([...new Set(entries.map((entry) => entry.engine))].sort());
  const operations = $derived([...new Set(entries.map((entry) => entry.operation))].sort());
  const shown = $derived.by(() => {
    const filtered = entries.filter(
      (entry) =>
        (!level || entry.level === level) &&
        (!engine || entry.engine === engine) &&
        (!operation || entry.operation === operation),
    );
    return order === "newest" ? filtered.reverse() : filtered;
  });

  function mergeEntry(entry: ActivityEntry) {
    if (!entries.some((existing) => existing.id === entry.id)) {
      entries = [...entries, entry].slice(-1000);
    }
  }

  onMount(() => {
    let active = true;
    let unlisten: (() => void) | undefined;
    void api
      .onActivity(mergeEntry)
      .then(async (stop) => {
        if (!active) {
          stop();
          return;
        }
        unlisten = stop;
        const initial = await api.activityEvents();
        if (active) {
          for (const entry of initial) mergeEntry(entry);
          problem = null;
        }
      })
      .catch((error) => {
        if (active) problem = String(error);
      });
    return () => {
      active = false;
      unlisten?.();
    };
  });

  $effect(() => {
    shown.length;
    order;
    if (autoScroll) {
      void tick().then(() => {
        list?.scrollTo({
          top: order === "newest" ? 0 : list.scrollHeight,
          behavior: "smooth",
        });
      });
    }
  });

  async function clear() {
    try {
      await api.clearActivityEvents();
      entries = [];
      message = locale.t("activity-log-cleared");
      problem = null;
    } catch (error) {
      problem = String(error);
    }
  }

  async function copy() {
    try {
      await navigator.clipboard.writeText(shown.map(formatEntry).join("\n\n"));
      message = locale.t("activity-log-copied");
      problem = null;
    } catch (error) {
      problem = String(error);
    }
  }

  async function exportLog(format: "txt" | "json") {
    try {
      const path = await save({
        defaultPath: `TDM-activity-log.${format}`,
        filters: [{ name: format.toUpperCase(), extensions: [format] }],
      });
      if (!path) return;
      await api.exportActivityLog(path, format, shown);
      message = locale.t("activity-log-exported");
      problem = null;
    } catch (error) {
      problem = String(error);
    }
  }

  function formatEntry(entry: ActivityEntry) {
    return `${entry.timestamp} [${entry.level}] ${entry.engine} / ${entry.operation}: ${entry.message}${
      entry.details ? `\n${entry.details}` : ""
    }`;
  }
</script>

<section class="panel activity-log-panel" aria-labelledby="activity-title">
    <header>
      <div>
        <h2 id="activity-title">{locale.t("activity-log-title")}</h2>
        <p>{locale.t("activity-log-count")}: {shown.length}</p>
      </div>
    </header>

    <div class="filters">
      <label>
        {locale.t("activity-log-level")}
        <select bind:value={level}>
          <option value="">{locale.t("activity-log-all")}</option>
          {#each ["INFO", "SUCCESS", "WARNING", "ERROR", "DEBUG"] as item}
            <option value={item}>{item}</option>
          {/each}
        </select>
      </label>
      <label>
        {locale.t("activity-log-engine")}
        <select bind:value={engine}>
          <option value="">{locale.t("activity-log-all")}</option>
          {#each engines as item}<option value={item}>{item}</option>{/each}
        </select>
      </label>
      <label>
        {locale.t("activity-log-operation")}
        <select bind:value={operation}>
          <option value="">{locale.t("activity-log-all")}</option>
          {#each operations as item}<option value={item}>{item}</option>{/each}
        </select>
      </label>
    </div>

    <div class="actions">
      <button class="btn" aria-pressed={order === "newest"} onclick={() => (order = order === "newest" ? "oldest" : "newest")}>
        <Icon name="arrow-up-down" size={15} />{locale.t(order === "newest" ? "activity-log-newest" : "activity-log-oldest")}
      </button>
      <button class="btn" aria-pressed={autoScroll} onclick={() => (autoScroll = !autoScroll)}>
        <Icon name={autoScroll ? "pause" : "play"} size={15} />{locale.t(autoScroll ? "activity-log-stop-scroll" : "activity-log-resume-scroll")}
      </button>
      <span class="spacer"></span>
      <button class="btn" onclick={() => void copy()}><Icon name="copy" size={15} />{locale.t("activity-log-copy")}</button>
      <button class="btn" onclick={() => void exportLog("txt")}><Icon name="download" size={15} />TXT</button>
      <button class="btn" onclick={() => void exportLog("json")}><Icon name="download" size={15} />JSON</button>
      <button class="btn danger" onclick={() => void clear()}><Icon name="trash-2" size={15} />{locale.t("activity-log-clear")}</button>
    </div>

    {#if problem}<p class="notice error" role="alert">{problem}</p>{/if}
    {#if message}<p class="notice" role="status">{message}</p>{/if}

    <div class="events" bind:this={list}>
      {#each shown as entry (entry.id)}
        <details class="event">
          <summary>
            <time>{entry.timestamp}</time>
            <span class="level {entry.level.toLowerCase()}">{entry.level}</span>
            <span class="engine">{entry.engine}</span>
            <span class="operation">{entry.operation}</span>
            <span class="description">{entry.message}</span>
          </summary>
          {#if entry.details}<pre>{entry.details}</pre>{/if}
        </details>
      {:else}
        <p class="empty">{locale.t("activity-log-empty")}</p>
      {/each}
    </div>
</section>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    width: 100%;
    overflow: hidden;
    border-top: 1px solid var(--line);
    background: var(--surface);
    color: var(--ink);
  }
  header,
  .actions,
  .filters {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 5px 12px;
    border-bottom: 1px solid var(--line);
  }
  header {
    justify-content: space-between;
  }
  h2,
  p {
    margin: 0;
  }
  h2 {
    font-size: 17px;
  }
  header p {
    color: var(--muted);
    font-size: 12px;
  }
  .filters {
    flex-wrap: wrap;
  }
  .filters label {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--muted);
    font-size: 12px;
  }
  select {
    max-width: 180px;
    border: 1px solid var(--line);
    border-radius: 6px;
    padding: 5px 8px;
    background: var(--surface);
    color: var(--ink);
    font: inherit;
  }
  .actions {
    flex-wrap: wrap;
  }
  .spacer {
    flex: 1;
  }
  .btn,
  .icon-button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 5px;
    border: 1px solid var(--line);
    border-radius: 6px;
    padding: 5px 8px;
    background: var(--surface);
    color: var(--ink);
    font: inherit;
    cursor: pointer;
  }
  .btn:hover,
  .icon-button:hover {
    background: var(--sel);
  }
  .danger {
    color: var(--red);
  }
  .notice {
    padding: 5px 12px;
    color: var(--green);
    font-size: 12px;
  }
  .notice.error {
    color: var(--red);
  }
  .events {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 5px 10px;
  }
  .event {
    border-bottom: 1px solid var(--line);
  }
  summary {
    display: grid;
    grid-template-columns: minmax(125px, auto) auto auto auto minmax(100px, 1fr);
    align-items: center;
    gap: 8px;
    padding: 7px 4px;
    cursor: pointer;
    font-size: 12px;
    list-style-position: inside;
  }
  time,
  .operation {
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
  .level {
    border-radius: 4px;
    padding: 2px 5px;
    font-size: 10px;
    font-weight: 700;
  }
  .info,
  .debug {
    color: var(--accent);
    background: var(--sunken);
  }
  .success {
    color: var(--green);
    background: var(--green-bg);
  }
  .warning {
    color: var(--amber);
    background: var(--amber-bg);
  }
  .error {
    color: var(--red);
    background: var(--red-bg);
  }
  .description {
    overflow-wrap: anywhere;
  }
  pre {
    max-height: 260px;
    overflow: auto;
    margin: 0 0 8px 25px;
    padding: 8px;
    border-radius: 6px;
    background: var(--sunken);
    font: 11px/1.5 ui-monospace, Consolas, monospace;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    direction: ltr;
    text-align: left;
  }
  .empty {
    padding: 30px;
    color: var(--muted);
    text-align: center;
  }
  @media (max-width: 720px) {
    summary {
      grid-template-columns: auto auto 1fr;
    }
    time {
      grid-column: span 2;
    }
    .operation {
      display: none;
    }
    .description {
      grid-column: 1 / -1;
    }
  }
</style>
