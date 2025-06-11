import React, { useEffect, useState } from 'react';
import { createApp } from 'vue';
import VueWidget from './VueWidget.vue';

const VueMount = () => {
  useEffect(() => {
    const mountPoint = document.getElementById('vue-widget-root');
    if (mountPoint && !mountPoint.hasChildNodes()) {
      const app = createApp(VueWidget);
      app.mount(mountPoint);
    }
  }, []);
  return <div id="vue-widget-root" style={{ minHeight: 200 }} />;
};

export default VueMount;
