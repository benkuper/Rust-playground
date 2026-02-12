import adapter from "@sveltejs/adapter-static";

const config = {
  kit: {
    adapter: adapter({
      fallback: "index.html"
    })
  },
  compilerOptions: {
    runes: true
  }
};

export default config;
