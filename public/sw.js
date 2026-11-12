// Offline support for Sabi no Nihongo.
//
// - Pages: network first, so a new deploy shows up right away; the cached
//   app shell is used when offline (any route, since routing is client-side).
// - Build files (hashed JS, WASM, CSS) and icons: cache first; they never
//   change under the same name. Files no longer used by the latest page are
//   removed after each successful page load.
// - Google Fonts: served from cache, refreshed in the background.
//
// Words are stored in localStorage, not here; this cache only holds the app.

const CACHE = "sabi-app-v1";
const FONTS = "sabi-fonts-v1";
const SHELL = new URL("./", self.registration.scope).href;

self.addEventListener("install", (event) => {
  event.waitUntil(
    caches
      .open(CACHE)
      .then((cache) => cache.addAll([SHELL, "manifest.webmanifest", "icons/icon-192.png"]))
      .then(() => self.skipWaiting()),
  );
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    caches
      .keys()
      .then((keys) => Promise.all(keys.filter((k) => k !== CACHE && k !== FONTS).map((k) => caches.delete(k))))
      .then(() => self.clients.claim()),
  );
});

self.addEventListener("fetch", (event) => {
  const req = event.request;
  if (req.method !== "GET") return;
  const url = new URL(req.url);

  if (req.mode === "navigate") {
    event.respondWith(page(req));
  } else if (url.origin === self.location.origin) {
    event.respondWith(cacheFirst(req));
  } else if (url.hostname === "fonts.googleapis.com" || url.hostname === "fonts.gstatic.com") {
    event.respondWith(staleWhileRevalidate(req));
  }
});

async function page(req) {
  const cache = await caches.open(CACHE);
  try {
    const res = await fetch(req);
    if (res.ok) {
      // Every route renders the same app, so keep one copy as the shell.
      const html = await res.clone().text();
      await cache.put(SHELL, res.clone());
      prune(cache, html);
    }
    return res;
  } catch (err) {
    const cached = await cache.match(SHELL);
    if (cached) return cached;
    throw err;
  }
}

async function cacheFirst(req) {
  const cache = await caches.open(CACHE);
  const cached = await cache.match(req);
  if (cached) return cached;
  const res = await fetch(req);
  if (res.ok) cache.put(req, res.clone());
  return res;
}

async function staleWhileRevalidate(req) {
  const cache = await caches.open(FONTS);
  const cached = await cache.match(req);
  const fresh = fetch(req)
    .then((res) => {
      if (res.ok || res.type === "opaque") cache.put(req, res.clone());
      return res;
    })
    .catch(() => cached);
  return cached || fresh;
}

// Drops build files that the latest page no longer links to.
async function prune(cache, html) {
  for (const req of await cache.keys()) {
    const path = new URL(req.url).pathname;
    const name = path.slice(path.lastIndexOf("/") + 1);
    if (/\.(js|wasm|css)$/.test(name) && name !== "sw.js" && !html.includes(name)) {
      await cache.delete(req);
    }
  }
}
