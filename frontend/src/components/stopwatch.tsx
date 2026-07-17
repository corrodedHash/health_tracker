import { useCallback, useEffect, useRef, useState } from "react";
import { Button } from "@/components/ui/button";

const LS_STARTED_AT = "stopwatch_started_at";
const LS_RUNNING = "stopwatch_running";

function formatDuration(ms: number): string {
  const totalSec = Math.floor(ms / 1000);
  const h = Math.floor(totalSec / 3600);
  const m = Math.floor((totalSec % 3600) / 60);
  const s = totalSec % 60;
  const pad = (n: number) => n.toString().padStart(2, "0");
  return `${pad(h)}:${pad(m)}:${pad(s)}`;
}

interface StopwatchProps {
  onStop: (startedAt: Date, durationMs: number) => void;
}

export function Stopwatch({ onStop }: StopwatchProps) {
  const [running, setRunning] = useState(false);
  const [elapsedMs, setElapsedMs] = useState(0);
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const startedAtRef = useRef<Date | null>(null);

  // Restore running state from localStorage on mount
  useEffect(() => {
    const storedRunning = localStorage.getItem(LS_RUNNING);
    const storedStartedAt = localStorage.getItem(LS_STARTED_AT);
    if (storedRunning === "true" && storedStartedAt) {
      startedAtRef.current = new Date(storedStartedAt);
      setRunning(true);
    }
  }, []);

  // Tick while running
  useEffect(() => {
    if (!running || !startedAtRef.current) {
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
        intervalRef.current = null;
      }
      return;
    }

    const tick = () => {
      setElapsedMs(Date.now() - startedAtRef.current!.getTime());
    };
    tick();
    intervalRef.current = setInterval(tick, 200);
    return () => {
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
        intervalRef.current = null;
      }
    };
  }, [running]);

  const handleStart = useCallback(() => {
    const now = new Date();
    startedAtRef.current = now;
    setElapsedMs(0);
    setRunning(true);
    localStorage.setItem(LS_STARTED_AT, now.toISOString());
    localStorage.setItem(LS_RUNNING, "true");
  }, []);

  const handleStop = useCallback(() => {
    if (!startedAtRef.current) return;
    const endedAt = new Date();
    const durationMs = endedAt.getTime() - startedAtRef.current.getTime();
    setRunning(false);
    setElapsedMs(durationMs);
    onStop(startedAtRef.current, durationMs);
    startedAtRef.current = null;
    localStorage.removeItem(LS_STARTED_AT);
    localStorage.removeItem(LS_RUNNING);
  }, [onStop]);

  // If the page was closed while running, restore the elapsed time
  useEffect(() => {
    if (running && startedAtRef.current) {
      setElapsedMs(Date.now() - startedAtRef.current.getTime());
    }
  }, [running]);

  return (
    <div className="flex items-center gap-3">
      <span className="font-mono text-lg tabular-nums min-w-[5.5rem]">
        {formatDuration(elapsedMs)}
      </span>
      {running ? (
        <Button type="button" variant="destructive" size="sm" onClick={handleStop}>
          Stop
        </Button>
      ) : (
        <Button type="button" variant="default" size="sm" onClick={handleStart}>
          Start
        </Button>
      )}
    </div>
  );
}
