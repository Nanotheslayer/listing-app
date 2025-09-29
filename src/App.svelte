<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";

  let name = "";
  let greetMsg = "";
  let apiUrl = "https://api.github.com/zen";
  let apiResponse = "";
  let loading = false;
  let apiKey = "";

  async function greet() {
    greetMsg = await invoke("greet", { name });
  }

  async function testApi() {
    loading = true;
    apiResponse = "";
    try {
      apiResponse = await invoke("test_api_call", { url: apiUrl });
    } catch (error) {
      apiResponse = `Error: ${error}`;
    } finally {
      loading = false;
    }
  }

  async function testG2GApi() {
    loading = true;
    apiResponse = "";
    try {
      apiResponse = await invoke("fetch_g2g_items", { apiKey });
    } catch (error) {
      apiResponse = `Error: ${error}`;
    } finally {
      loading = false;
    }
  }
</script>

<main class="min-h-screen bg-gradient-to-br from-blue-50 to-indigo-100 p-8">
  <div class="max-w-4xl mx-auto">
    <h1 class="text-5xl font-bold mb-8 text-center text-indigo-900">
      🎮 G2G App
    </h1>

    <!-- Greet Section -->
    <div class="bg-white rounded-lg shadow-lg p-6 mb-6">
      <h2 class="text-2xl font-semibold mb-4 text-gray-800">
        👋 Greet Test
      </h2>
      <div class="flex gap-3">
        <input
          class="flex-1 px-4 py-3 border-2 border-gray-300 rounded-lg focus:border-indigo-500 focus:outline-none transition"
          placeholder="Enter your name..."
          bind:value={name}
        />
        <button
          class="px-6 py-3 bg-indigo-500 text-white font-semibold rounded-lg hover:bg-indigo-600 active:scale-95 transition"
          on:click={greet}
        >
          Greet
        </button>
      </div>
      {#if greetMsg}
        <div class="mt-4 p-4 bg-green-50 border-l-4 border-green-500 rounded">
          <p class="text-green-800">{greetMsg}</p>
        </div>
      {/if}
    </div>

    <!-- API Test Section -->
    <div class="bg-white rounded-lg shadow-lg p-6 mb-6">
      <h2 class="text-2xl font-semibold mb-4 text-gray-800">
        🌐 API Test
      </h2>
      <div class="flex gap-3 mb-4">
        <input
          class="flex-1 px-4 py-3 border-2 border-gray-300 rounded-lg focus:border-blue-500 focus:outline-none transition"
          placeholder="API URL..."
          bind:value={apiUrl}
        />
        <button
          class="px-6 py-3 bg-blue-500 text-white font-semibold rounded-lg hover:bg-blue-600 active:scale-95 transition disabled:opacity-50 disabled:cursor-not-allowed"
          on:click={testApi}
          disabled={loading}
        >
          {loading ? "⏳ Loading..." : "Test API"}
        </button>
      </div>
      {#if apiResponse}
        <pre class="mt-4 p-4 bg-gray-50 border border-gray-200 rounded-lg overflow-auto text-sm">{apiResponse}</pre>
      {/if}
    </div>

    <!-- G2G API Section -->
    <div class="bg-white rounded-lg shadow-lg p-6">
      <h2 class="text-2xl font-semibold mb-4 text-gray-800">
        🎯 G2G API Test
      </h2>
      <div class="flex gap-3">
        <input
          type="password"
          class="flex-1 px-4 py-3 border-2 border-gray-300 rounded-lg focus:border-purple-500 focus:outline-none transition"
          placeholder="Enter G2G API Key..."
          bind:value={apiKey}
        />
        <button
          class="px-6 py-3 bg-purple-500 text-white font-semibold rounded-lg hover:bg-purple-600 active:scale-95 transition disabled:opacity-50 disabled:cursor-not-allowed"
          on:click={testG2GApi}
          disabled={loading || !apiKey}
        >
          {loading ? "⏳ Loading..." : "Test G2G"}
        </button>
      </div>
      {#if apiResponse && apiKey}
        <pre class="mt-4 p-4 bg-purple-50 border border-purple-200 rounded-lg overflow-auto text-sm">{apiResponse}</pre>
      {/if}
    </div>
  </div>
</main>
