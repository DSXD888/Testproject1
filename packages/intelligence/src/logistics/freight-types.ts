export type ShippingMode =
  | "sea_lcl"
  | "sea_fcl"
  | "air_general"
  | "air_express"
  | "multimodal";

export type PackagingState = "palletized" | "loose_cartons" | "unknown";

export type BatteryState =
  | "none"
  | "loose"
  | "in_device"
  | "lithium_unknown";

export type Urgency = "standard" | "expedited" | "critical";

export interface ModeSelectionFacts {
  weightKg: number;
  volumeM3: number;
  valueGbpPence: number;
  urgency: Urgency;
  productType: string;
  hazmat: boolean;
  oversized: boolean;
  fragile: boolean;
  reefer: boolean;
  batteries: BatteryState;
  packagingState: PackagingState;
}

export interface ModeSelection {
  mode: ShippingMode;
  confidenceBps: number;
  hardLocksTriggered: HardLock[];
  reason: string;
  fallbackBoundaryUsed: boolean;
}

export type HardLock =
  | { kind: "hazmat"; details: string }
  | { kind: "battery"; details: string }
  | { kind: "reefer"; details: string }
  | { kind: "oversized"; details: string }
  | { kind: "fragile"; details: string };

export interface FreightEstimate {
  provider: "freightos" | "searates" | "manual";
  mode: ShippingMode;
  lowGbpPence: number | null;
  baseGbpPence: number | null;
  highGbpPence: number | null;
  evidenceTranscriptPath?: string;
  failureCode?: string;
  checkedAt: string;
}

export interface PackingScenario {
  packagingState: PackagingState;
  scenariosToRun: ("palletized" | "loose_cartons")[];
}
