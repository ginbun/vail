import type { App } from 'vue';
import { use } from 'echarts/core';
import QueryHeader from './query-header/index.vue';
import { CanvasRenderer } from 'echarts/renderers';
import { BarChart, LineChart, PieChart, RadarChart } from 'echarts/charts';
import { DataZoomComponent, GraphicComponent, GridComponent, LegendComponent, TooltipComponent, } from 'echarts/components';
import Breadcrumb from './app/breadcrumb/index.vue';
import Chart from './view/chart/index.vue';
import CardList from './view/card-list/index.vue';
import Editor from './view/editor/index.vue';
import TabRouter from './view/tab-router/index.vue';

use([
  CanvasRenderer,
  BarChart,
  LineChart,
  PieChart,
  RadarChart,
  GridComponent,
  TooltipComponent,
  LegendComponent,
  DataZoomComponent,
  GraphicComponent,
]);

export default {
  install(Vue: App) {
    Vue.component('Chart', Chart);
    Vue.component('Breadcrumb', Breadcrumb);
    Vue.component('QueryHeader', QueryHeader);
    Vue.component('CardList', CardList);
    Vue.component('Editor', Editor);
    Vue.component('TabRouter', TabRouter);
  },
};
