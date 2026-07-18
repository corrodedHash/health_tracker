import ReactECharts from "echarts-for-react";
import { useMemo } from "react";
import type { WeightSession } from "@/types";

interface WeightOverTimeChartProps {
  sessions: Array<{ session: { id: string; startedAt: Date }; weight: WeightSession }>;
}

export function WeightOverTimeChart({ sessions }: WeightOverTimeChartProps) {
  const option = useMemo(() => {
    const sorted = [...sessions].sort(
      (a, b) => a.session.startedAt.getTime() - b.session.startedAt.getTime(),
    );
    const dates = sorted.map((s) => s.session.startedAt.toLocaleDateString());
    const weights = sorted.map((s) => s.weight.weight_g / 1000);
    return {
      tooltip: { trigger: "axis" },
      xAxis: { type: "category", data: dates, axisLabel: { rotate: 30 } },
      yAxis: { type: "value", name: "kg", scale: true },
      series: [
        {
          name: "Weight",
          type: "line",
          data: weights,
          smooth: true,
          areaStyle: {},
        },
      ],
      grid: { left: 48, right: 24, bottom: 56, top: 24 },
    };
  }, [sessions]);

  if (sessions.length === 0) {
    return (
      <p className="text-sm text-muted-foreground">
        No weight sessions yet — log one below to see the chart.
      </p>
    );
  }

  return <ReactECharts option={option} style={{ height: 280, width: "100%" }} />;
}