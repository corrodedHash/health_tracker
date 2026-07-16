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
  exercise_name: string;
  weight_kg: number;
  sets: number;
  reps: number;
  quality: number | null;
}

export interface CoreSession {
  session_id: string;
  exercise_name: string;
  duration_secs: number;
  quality: number | null;
}

export interface RunningSummary {
  sessionId: string;
  distanceM: number;
  hasGpx: boolean;
}

export interface WeightCreate {
  exercise_name: string;
  weight_kg: number;
  sets: number;
  reps: number;
  quality: number | null;
}