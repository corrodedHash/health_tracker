import ReactECharts from "echarts-for-react";
import { useQuery } from "@tanstack/react-query";
import { useMemo } from "react";
import { getRunningSummary, listSessions } from "@/lib/api";

export function RunningDistanceChart() {
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

    return {
      tooltip: {
        trigger: "axis",
        formatter: (params: Array<{ value: number; axisValue: string }>) => {
          const p = params[0];
          return `${p.axisValue}<br/>Distance: ${p.value.toFixed(2)} km`;
        },
      },
      xAxis: {
        type: "category",
        data: sorted.map((s) => s.session.startedAt.toLocaleDateString()),
        axisLabel: { rotate: 30 },
      },
      yAxis: { type: "value", name: "km", scale: true },
      series: [
        {
          name: "Distance",
          type: "bar",
          data: sorted.map((s) => s.running.distanceM / 1000),
        },
      ],
      grid: { left: 48, right: 24, bottom: 56, top: 24 },
    };
  }, [runningQ.data]);

  if (sessionsQ.isLoading) {
    return <p className="text-sm text-muted-foreground">Loading running data…</p>;
  }

  if (!runningQ.data || runningQ.data.length === 0) {
    return (
      <p className="text-sm text-muted-foreground">
        No running sessions yet — log one to see distance data.
      </p>
    );
  }

  return <ReactECharts option={option} style={{ height: 280, width: "100%" }} />;
}
