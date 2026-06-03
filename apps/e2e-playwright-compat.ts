import { expect as playwrightExpect, test as playwrightTest } from '@playwright/test';
import type { Page, Route } from '@playwright/test';

const isBareBunTest =
  'Bun' in globalThis &&
  !process.argv.some((arg) => arg.includes('playwright'));

const noop = () => undefined;
const noopDescribe = Object.assign((_title: string, _body?: () => void) => undefined, {
  configure: noop,
  fixme: noop,
  only: (_title: string, _body?: () => void) => undefined,
  skip: (_title: string, _body?: () => void) => undefined,
});
const noopTest = Object.assign((_title: string, _body?: unknown) => undefined, {
  afterAll: noop,
  afterEach: noop,
  beforeAll: noop,
  beforeEach: noop,
  describe: noopDescribe,
  fixme: noop,
  only: (_title: string, _body?: unknown) => undefined,
  skip: (_title: string, _body?: unknown) => undefined,
  slow: noop,
  step: noop,
  use: noop,
});

export const test = (isBareBunTest ? noopTest : playwrightTest) as typeof playwrightTest;
export const expect = playwrightExpect;
export type { Page, Route };
