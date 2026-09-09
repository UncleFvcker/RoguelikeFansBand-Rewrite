/// <reference types="vite/client" />

interface Window {
  __rfbPrepareSupplyE2e?: (amount: number) => Promise<void>;
  __rfbPrepareLifeForceE2e?: (seed: number) => Promise<void>;
}
