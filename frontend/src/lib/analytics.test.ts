import { afterEach, describe, expect, it } from 'vitest';
import { BEACON_SRC, loadWebAnalytics } from './analytics';

const beacons = () => document.head.querySelectorAll(`script[src="${BEACON_SRC}"]`);

describe('loadWebAnalytics', () => {
  afterEach(() => beacons().forEach((script) => script.remove()));

  it('adds the beacon with the token', () => {
    loadWebAnalytics('madeuptoken123');
    const script = beacons()[0] as HTMLScriptElement;
    expect(script.defer).toBe(true);
    expect(JSON.parse(script.dataset.cfBeacon!)).toEqual({ token: 'madeuptoken123' });
  });

  it('adds it only once', () => {
    loadWebAnalytics('madeuptoken123');
    loadWebAnalytics('madeuptoken123');
    expect(beacons()).toHaveLength(1);
  });
});
