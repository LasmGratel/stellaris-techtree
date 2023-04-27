<template>
  <v-container>
    <p>GG</p>
    <v-switch inset color="primary" v-model="showIndependentTech" label="显示独立科研"></v-switch>
    <div id="graph"></div>
  </v-container>
</template>
<script>
import { parseJSON } from "@msagl/parser"
import { Renderer } from "@msagl/renderer"

export default {
  name: 'TechTree',

  data: () => ({
    showIndependentTech: false
  }),

  methods: {
    traverseTree() {
    }

  },

  mounted() {
    const renderer = new Renderer(document.getElementById("graph"), null);
    const graph = parseJSON({
      nodes: [{ id: 'kspacey' }, { id: 'swilliams' }, { id: 'kbacon' }, { id: 'bpitt' }, { id: 'hford' }, { id: 'lwilson' }],
      edges: [
        { source: 'kspacey', target: 'swilliams' },
        { source: 'swilliams', target: 'kbacon' },
        { source: 'bpitt', target: 'kbacon' },
        { source: 'hford', target: 'lwilson' },
        { source: 'lwilson', target: 'kbacon' },
      ],
    })
    renderer.setGraph(graph)
  },
}


</script>
