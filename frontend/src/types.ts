export type ExerciseKind = "weight" | "core" | "running";

export interface ExerciseSession {
  id: string;
  userId: string;
  kind: ExerciseKind;
  startedAt: Date;
  durationMs: number;
  notes: string | null;
  createdAt: Date;
}

export interface NewExerciseSession {
  kind: ExerciseKind;
  startedAt: Date;
  durationMs: number;
  notes: string | null;
}

export interface WeightSession {
  session_id: string;
  weight_kg: number;
  sets: number;
  quality: number | null;
}

export interface CoreSession {
  session_id: string;
  quality: number | null;
}

export interface RunningSummary {
  sessionId: string;
  distanceM: number;
  quality: number | null;
  movingDistanceM: number | null;
  movingTime: number | null;
  hasGpx: boolean;
}

export interface WeightCreate {
  weight_kg?: number;
  sets?: number;
  quality: number | null;
}

export interface CoreCreate {
  quality: number | null;
}

export interface RunningCreate {
  distance_m: number;
  quality: number | null;
}
