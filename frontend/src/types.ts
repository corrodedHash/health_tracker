export type ExerciseKind = "weight" | "core" | "running" | "custom";

export interface ExerciseSession {
  id: string;
  userId: string;
  kind: ExerciseKind;
  startedAt: Date;
  durationMs: number;
  notes: string | null;
  quality: number | null;
  createdAt: Date;
}

export interface NewExerciseSession {
  kind: ExerciseKind;
  startedAt: Date;
  durationMs: number;
  notes: string | null;
  quality: number | null;
}

export interface WeightSession {
  session_id: string;
  weight_g: number;
  sets: number;
}

export interface CoreSession {
  session_id: string;
}

export interface RunningSummary {
  sessionId: string;
  distanceM: number;
  movingDistanceM: number | null;
  movingTime: number | null;
  hasGpx: boolean;
}

export interface WeightCreate {
  weight_g: number;
  sets: number;
}

// eslint-disable-next-line @typescript-eslint/no-empty-object-type
export interface CoreCreate {
}

export interface RunningCreate {
  distance_m: number;
}
