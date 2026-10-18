// A steady tick for web/net.js, from a worker: a tab in the background gets
// no animation frames and its own timers slow to about once a second, but
// a worker's timer keeps running, so a host left in the background goes on
// sending its boats and judging the match (SECOND-ORDER R3.51). The page
// posts the interval (web/online-config.js `pumpMs`); each tick posts back.
let timer = null;

onmessage = (e) => {
  if (timer !== null) clearInterval(timer);
  timer = setInterval(() => postMessage(0), e.data);
};
