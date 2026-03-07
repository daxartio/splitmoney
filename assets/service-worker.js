const CACHE_NAME = "splitmoney-cache-v1";

const BASE_PATH = (() => {
    const match = self.location.pathname.match(/^(.*)assets\/service-worker\.js$/);
    if (match && match[1]) {
        const path = match[1];
        return path.endsWith("/") ? path : `${path}/`;
    }
    return "/";
})();

const CORE_ASSETS = [
    BASE_PATH,
    `${BASE_PATH}index.html`,
    `${BASE_PATH}assets/main.css`,
    `${BASE_PATH}assets/manifest.json`,
    `${BASE_PATH}assets/sw-register.js`,
    `${BASE_PATH}assets/icon-192.png`,
    `${BASE_PATH}assets/icon-512.png`,
    `${BASE_PATH}assets/apple-touch-icon.png`,
];

self.addEventListener("install", (event) => {
    event.waitUntil(
        caches
            .open(CACHE_NAME)
            .then((cache) => cache.addAll(CORE_ASSETS))
            .then(() => self.skipWaiting())
    );
});

self.addEventListener("activate", (event) => {
    event.waitUntil(
        caches
            .keys()
            .then((keys) =>
                Promise.all(
                    keys
                        .filter((key) => key !== CACHE_NAME)
                        .map((key) => caches.delete(key))
                )
            )
            .then(() => self.clients.claim())
    );
});

self.addEventListener("fetch", (event) => {
    if (event.request.method !== "GET") {
        return;
    }

    const requestUrl = new URL(event.request.url);
    if (requestUrl.origin !== self.location.origin) {
        return;
    }

    if (!requestUrl.pathname.startsWith(BASE_PATH)) {
        return;
    }

    event.respondWith(
        caches.match(event.request).then((cached) => {
            if (cached) {
                return cached;
            }

            return fetch(event.request)
                .then((response) => {
                    if (response && response.status === 200) {
                        const responseCopy = response.clone();
                        caches.open(CACHE_NAME).then((cache) => {
                            cache.put(event.request, responseCopy);
                        });
                    }
                    return response;
                })
                .catch(() => {
                    if (event.request.mode === "navigate") {
                        return caches.match(`${BASE_PATH}index.html`);
                    }
                    return cached;
                });
        })
    );
});
