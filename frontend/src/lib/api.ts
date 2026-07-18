import axios from "axios";
import dayjs from "dayjs";
import type {
  CoreCreate,
  CoreSession,
  ExerciseKind,
  ExerciseSession,
  NewExerciseSession,
  RunningCreate,
  RunningSummary,
  WeightCreate,
  WeightSession,
} from "@/types";

export interface SessionDto {
  id: string;
  user_id: string;
  kind: "weight" | "core" | "running" | "custom";
  started_at: string;
  duration_secs: number;
  notes: string | null;
  quality: number | null;
  created_at: string;
}

export interface WeightSessionDto {
  session_id: string;
  weight_g: number;
  sets: number;
}

export interface RunningSessionDto {
  session_id: string;
  distance_m: number;
  moving_distance_m: number | null;
  moving_time: number | null;
  has_gpx: boolean;
}

const toSession = (dto: SessionDto): ExerciseSession => ({
  id: dto.id,
  userId: dto.user_id,
  kind: dto.kind,
  startedAt: dayjs.utc(dto.started_at).toDate(),
  durationMs: dto.duration_secs * 1000,
  notes: dto.notes,
  quality: dto.quality,
  createdAt: dayjs.utc(dto.created_at).toDate(),
});

export async function listSessions(
  kind?: ExerciseKind,
  limit?: number,
  offset?: number,
): Promise<ExerciseSession[]> {
  const params: Record<string, unknown> = {};
  if (kind) params.kind = kind;
  if (limit !== undefined) params.limit = limit;
  if (offset !== undefined) params.offset = offset;
  const resp = await axios.get<SessionDto[]>("/api/exercise-sessions", { params });
  return resp.data.map(toSession);
}

export async function createSession(
  body: NewExerciseSession,
): Promise<ExerciseSession> {
  const resp = await axios.post<SessionDto>("/api/exercise-sessions", {
    kind: body.kind,
    started_at: body.startedAt.toISOString(),
    duration_secs: body.durationMs / 1000,
    notes: body.notes,
    quality: body.quality,
  }, {
    headers: { "Content-Type": "application/json" },
  });
  return toSession(resp.data);
}

export async function createWeightDetails(
  sessionId: string,
  data: WeightCreate,
): Promise<void> {
  await axios.post(`/api/exercise-sessions/${sessionId}/weight`, data);
}

export async function getWeightDetails(
  sessionId: string,
): Promise<WeightSession> {
  const resp = await axios.get<WeightSessionDto>(
    `/api/exercise-sessions/${sessionId}/weight`,
  );
  return { ...resp.data };
}

export async function createCoreDetails(
  sessionId: string,
  data: CoreCreate,
): Promise<void> {
  await axios.post(`/api/exercise-sessions/${sessionId}/core`, data);
}

export async function getCoreDetails(
  sessionId: string,
): Promise<CoreSession> {
  const resp = await axios.get<CoreSession>(
    `/api/exercise-sessions/${sessionId}/core`,
  );
  return resp.data;
}

export async function createRunningDetails(
  sessionId: string,
  data: RunningCreate,
): Promise<void> {
  await axios.post(`/api/exercise-sessions/${sessionId}/running`, data);
}

export async function getRunningSummary(
  sessionId: string,
): Promise<RunningSummary> {
  const resp = await axios.get<RunningSessionDto>(
    `/api/exercise-sessions/${sessionId}/running`,
  );
  return {
    sessionId: resp.data.session_id,
    distanceM: resp.data.distance_m,
    movingDistanceM: resp.data.moving_distance_m,
    movingTime: resp.data.moving_time,
    hasGpx: resp.data.has_gpx,
  };
}

export async function deleteSession(sessionId: string): Promise<void> {
  await axios.delete(`/api/exercise-sessions/${sessionId}`);
}

export async function checkAuth(): Promise<boolean> {
  const resp = await axios.get<{ authenticated: boolean }>("/auth/status");
  return resp.data.authenticated;
}
