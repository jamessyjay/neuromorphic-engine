import { initUIEventListeners, buildLegend } from './ui.js';
import { initRenderer } from './render.js';
import { connect } from './network.js';

window.addEventListener('load', () => {
    initUIEventListeners();
    buildLegend();
    initRenderer();
    connect(window.location.host);
});
