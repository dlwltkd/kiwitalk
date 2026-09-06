import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { setImmediate } from 'node:timers/promises';
import test from 'node:test';
import ts from 'typescript';

const require = createRequire(import.meta.url);
const solid = require('solid-js/dist/solid.cjs');
const source = readFileSync(new URL('../../frontend/src/pages/main/channel/chat/_hooks/useMessageList.ts', import.meta.url), 'utf8');
const compiled = ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 },
}).outputText;

const complete = { complete: true, stopReason: 'reachedTarget', fetchedCount: 0, pageCount: 1, cachedCount: 0, gapCount: 0 };
const chat = (id) => ({ logId: String(9007199254740993n + BigInt(id)), senderId: '1', content: `message ${id}` });

function harness(api) {
  const [channelId, setChannelId] = solid.createSignal('room-a');
  const [event, setEvent] = solid.createSignal(null);
  const exports = {};
  const dependencies = {
    'solid-js': solid,
    '@/api/client': api,
    '@/pages/main/_hooks': { useReady: () => () => true, useChannelEvent: () => event },
  };
  new Function('require', 'exports', compiled)((name) => {
    assert.ok(name in dependencies, `unexpected dependency: ${name}`);
    return dependencies[name];
  }, exports);
  let dispose;
  const transcript = solid.createRoot((cleanup) => {
    dispose = cleanup;
    return exports.useMessageList(channelId);
  });
  return { transcript, setChannelId, setEvent, dispose, messages: () => transcript.messageGroups().flat() };
}

async function settle(until) {
  for (let i = 0; i < 50; i++) {
    await setImmediate();
    if (until()) return;
  }
  assert.fail('history request did not settle');
}

test('backfill beyond 200 messages does not skip between sparse cached rows', async (t) => {
  let cache = [chat(301), chat(1)];
  const cursors = [];
  const h = harness({
    loadChat: async (_id, count, from) => {
      cursors.push(from);
      return cache.filter((row) => !from || BigInt(row.logId) < BigInt(from)).slice(0, count);
    },
    syncChannelHistory: async () => {
      cache = Array.from({ length: 301 }, (_, i) => chat(301 - i));
      return complete;
    },
  });
  t.after(h.dispose);
  await settle(() => cursors.length === 2 && !h.transcript.loading());
  assert.equal(h.transcript.isEnd(), false);
  h.transcript.loadMore();
  await settle(() => cursors.length === 3 && !h.transcript.loading());
  assert.equal(cursors[2], chat(102).logId);
  assert.deepEqual(h.messages().map((row) => row.logId), cache.map((row) => row.logId));
  assert.equal(h.transcript.isEnd(), true);
});

test('incomplete history can continue without reopening the room', async (t) => {
  let syncs = 0;
  const h = harness({
    loadChat: async () => [chat(syncs + 1)],
    syncChannelHistory: async () => {
      syncs++;
      return syncs === 1 ? { ...complete, complete: false, stopReason: 'pageLimit' } : complete;
    },
  });
  t.after(h.dispose);
  await settle(() => syncs === 1 && !h.transcript.loading());
  assert.equal(h.transcript.canSyncHistory(), true);
  assert.equal(h.transcript.isEnd(), true);
  h.transcript.syncMore();
  await settle(() => syncs === 2 && !h.transcript.loading());
  assert.equal(h.transcript.canSyncHistory(), false);
  assert.equal(h.transcript.error(), null);
});

test('late room response cannot replace the newly selected room', async (t) => {
  let finishOldRoom;
  const oldRoom = new Promise((resolve) => { finishOldRoom = resolve; });
  const h = harness({
    loadChat: async (id) => id === 'room-a' ? oldRoom : [chat(20)],
    syncChannelHistory: async () => complete,
  });
  t.after(h.dispose);
  await settle(() => h.transcript.loading());
  h.setChannelId('room-b');
  await settle(() => h.messages().length > 0 && !h.transcript.loading());
  finishOldRoom([chat(1)]);
  await setImmediate();
  assert.deepEqual(h.messages().map((row) => row.logId), [chat(20).logId]);
});

test('out-of-order live messages and duplicate pages remain sorted and unique', async (t) => {
  const h = harness({
    loadChat: async () => [chat(10), chat(8), chat(8)],
    syncChannelHistory: async () => complete,
  });
  t.after(h.dispose);
  await settle(() => h.messages().length > 0 && !h.transcript.loading());
  h.setEvent({ channelId: 'room-a', type: 'Chat', content: { chat: chat(9) } });
  assert.deepEqual(h.messages().map((row) => row.logId), [10, 9, 8].map((id) => chat(id).logId));
});

test('scrolling past cached history continues a partial sync without a retry click', async (t) => {
  let syncs = 0;
  const h = harness({
    loadChat: async () => Array.from({ length: syncs + 1 }, (_, i) => chat(syncs + 1 - i)),
    syncChannelHistory: async () => {
      syncs++;
      return syncs === 1 ? { ...complete, complete: false, stopReason: 'pageLimit', fetchedCount: 1 } : complete;
    },
  });
  t.after(h.dispose);
  await settle(() => syncs === 1 && !h.transcript.loading());
  assert.equal(h.transcript.isEnd(), false);
  h.transcript.loadMore();
  await settle(() => syncs === 2 && !h.transcript.loading());
  assert.deepEqual(h.messages().map((row) => row.logId), [3, 2, 1].map((id) => chat(id).logId));
  assert.equal(h.transcript.isEnd(), true);
});

test('an empty or stalled history response does not cause an automatic retry loop', async (t) => {
  let syncs = 0;
  const h = harness({
    loadChat: async () => [chat(1)],
    syncChannelHistory: async () => {
      syncs++;
      return { ...complete, complete: false, stopReason: 'emptyBatch', fetchedCount: 0 };
    },
  });
  t.after(h.dispose);
  await settle(() => syncs === 1 && !h.transcript.loading());
  for (let i = 0; i < 10; i++) h.transcript.loadMore();
  await setImmediate();
  assert.equal(syncs, 1);
  assert.equal(h.transcript.isEnd(), true);
  assert.equal(h.transcript.canSyncHistory(), true);
});
