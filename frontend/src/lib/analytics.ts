/** Cloudflare's Web Analytics beacon. */
export const BEACON_SRC = 'https://static.cloudflareinsights.com/beacon.min.js';

/** Adds Cloudflare's Web Analytics beacon for `token`, once. */
export function loadWebAnalytics(token: string, doc: Document = document) {
  if (doc.querySelector(`script[src="${BEACON_SRC}"]`)) {
    return;
  }
  const script = doc.createElement('script');
  script.defer = true;
  script.src = BEACON_SRC;
  script.dataset.cfBeacon = JSON.stringify({ token });
  doc.head.append(script);
}
