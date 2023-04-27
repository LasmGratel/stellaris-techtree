<template>
  <v-app>
    <v-navigation-drawer app>
      <!-- -->
    </v-navigation-drawer>

    <v-app-bar app>
      <!-- -->
    </v-app-bar>
    <v-main>

      <TechTree />
    </v-main>

    <v-footer app>
      <!-- -->
    </v-footer>
  </v-app>
</template>

<script>
import HelloWorld from './components/HelloWorld.vue'
import TechTree from './components/TechTree.vue'

export default {
  name: 'App',

  components: {
    HelloWorld,
    TechTree,
  },

  data: () => ({
    //
  }),
}
</script>
