import ReactECharts from "echarts-for-react";
import { useQuery } from "@tanstack/react-query";
import { useMemo } from "react";
import { getRunningSummary, listSessions } from "@/lib/api";

function formatPace(minPerKm: number): string {
  const m = Math.floor(minPerKm);
  const s = Math.round((minPerKm - m) * 60);
  return `${m}:${String(s).padStart(2, "0")}`;
}

export function RunningPaceChart() {
  const sessionsQ = useQuery({
    queryKey: ["sessions", "running"],
    queryFn: () => listSessions("running"),
    retry: false,
  });

  const runningQ = useQuery({
    queryKey: ["running-details", sessionsQ.data?.map((s) => s.id).join(",") ?? ""],
    queryFn: async () => {
      const sessions = sessionsQ.data ?? [];
      const entries = await Promise.all(
        sessions.map(async (session) => {
          try {
            const running = await getRunningSummary(session.id);
            return { session, running };
          } catch {
            return null;
          }
        }),
      );
      return entries.filter((e): e is NonNullable<typeof e> => e !== null);
    },
    enabled: sessionsQ.data !== undefined && sessionsQ.data.length > 0,
    retry: false,
  });

  const option = useMemo(() => {
    const entries = runningQ.data ?? [];
    const sorted = [...entries].sort(
      (a, b) => a.session.startedAt.getTime() - b.session.startedAt.getTime(),
    );
    const data = sorted
      .filter((e) => e.running.distanceM > 0)
      .map((e) => ({
        date: e.session.startedAt.toLocaleDateString(),
        pace: e.session.durationMs / (e.running.distanceM * 60),
      }));

    return {
      tooltip: {
        trigger: "axis",
        formatter: (params: Array<{ value: number; axisValue: string }>) => {
          const p = params[0];
          return `${p.axisValue}<br/>Pace: ${formatPace(p.value)} /km`;
        },
      },
      xAxis: {
        type: "category",
        data: data.map((d) => d.date),
        axisLabel: { rotate: 30 },
      },
      yAxis: {
        type: "value",
        name: "min/km",
        axisLabel: { formatter: (v: number) => formatPace(v) },
        inverse: true,
        scale: true,
      },
      series: [
        {
          name: "Pace",
          type: "line",
          data: data.map((d) => d.pace),
          smooth: true,
          areaStyle: {},
        },
      ],
      grid: { left: 64, right: 24, bottom: 56, top: 24 },
    };
  }, [runningQ.data]);

  if (sessionsQ.isLoading) {
    return <p className="text-sm text-muted-foreground">Loading running data…</p>;
  }

  if (!runningQ.data || runningQ.data.length === 0) {
    return (
      <p className="text-sm text-muted-foreground">
        No running sessions yet — log one to see pace data.
      </p>
    );
  }

  return <ReactECharts option={option} style={{ height: 280, width: "100%" }} />;
}
