import adapter from "@sveltejs/adapter-static";

const config = {
  kit: {
    alias: {
      $golden_ui: "../crates/golden_ui/src/lib"
    },
    adapter: adapter({
      fallback: "index.html"
    })
  },
  compilerOptions: {
    runes: true
  }
};

export default config;
