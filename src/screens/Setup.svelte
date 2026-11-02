<script lang="ts">
  // quiz-setup.feature (R15 to R19)
  import { CircleAlert, Play } from "@lucide/svelte";
  import { api, errorMessage, type Id, type ModuleRow, type QuizView, type TopicRow } from "../lib/api";
  import PageHeader from "../components/PageHeader.svelte";
  import Seg from "../components/Seg.svelte";

  let { preselected = [], onStarted }: { preselected?: Id[]; onStarted: (view: QuizView) => void } = $props();

  let topics = $state<TopicRow[]>([]);
  let modules = $state<ModuleRow[]>([]);
  // svelte-ignore state_referenced_locally
  let chosen = $state<Id[]>([...preselected]);

  let exam = $state(false);
  let setNumber = $state(false);
  let count = $state(20);
  let even = $state(false);
  let shuffleQuestions = $state(false);
  let shuffleOptions = $state(false);
  let useTotal = $state(false);
  let totalMinutes = $state(20);
  let usePerQuestion = $state(false);
  let perQuestionSeconds = $state(45);
  let easy = $state(false);

  let error = $state("");
  let starting = $state(false);

  const available = $derived(topics.filter((topic) => chosen.includes(topic.id)).reduce((sum, topic) => sum + topic.question_count, 0));
  const questionCount = $derived(setNumber ? count : available);
  const moduleName = (id: Id | null) => modules.find((module) => module.id === id)?.name ?? "No module";

  function toggle(id: Id) {
    chosen = chosen.includes(id) ? chosen.filter((topic) => topic !== id) : [...chosen, id];
  }

  async function start() {
    error = "";
    if (chosen.length === 0) {
      error = "Pick at least one topic to start.";
      return;
    }
    starting = true;
    try {
      const view = await api.startQuiz({
        topics: topics.map((topic) => topic.id).filter((id) => chosen.includes(id)),
        count: setNumber ? Number(count) : null,
        even,
        shuffle_questions: shuffleQuestions,
        shuffle_options: shuffleOptions,
        exam,
        total_minutes: useTotal ? Number(totalMinutes) : null,
        per_question_seconds: usePerQuestion ? Number(perQuestionSeconds) : null,
        easy,
      });
      onStarted(view);
    } catch (failure) {
      error = errorMessage(failure);
    } finally {
      starting = false;
    }
  }

  Promise.all([api.topics(), api.modules()]).then(([topicRows, moduleRows]) => {
    topics = topicRows;
    modules = moduleRows;
  });
</script>

{#snippet row(label: string, help: string, control: import("svelte").Snippet)}
  <div class="flex items-center justify-between gap-8 px-6 py-4">
    <div><p class="text-sm font-semibold">{label}</p>{#if help}<p class="mt-1 text-sm text-muted">{help}</p>{/if}</div>
    <div class="shrink-0">{@render control()}</div>
  </div>
{/snippet}

<div class="mx-auto max-w-6xl px-12 pb-24">
  <PageHeader title="New quiz" subtitle="Pick what to practise and how" />
  <div class="grid grid-cols-[1fr_320px] items-start gap-8">
    <div class="space-y-8">
      <section>
        <h2 class="eyebrow mb-3">Topics</h2>
        <div class="panel divide-y">
          {#each topics as topic (topic.id)}
            <label class="flex cursor-pointer items-center gap-4 px-6 py-3 hover:bg-base-100">
              <input type="checkbox" class="checkbox checkbox-sm" checked={chosen.includes(topic.id)} onchange={() => toggle(topic.id)} />
              <span class="flex-1"><span class="text-sm font-semibold">{topic.name}</span> <span class="text-sm text-muted">· {moduleName(topic.module)}</span></span>
              <span class="text-xs tabular-nums text-muted">{topic.question_count} questions</span>
            </label>
          {:else}
            <p class="px-6 py-4 text-sm text-muted">Create a topic and write some questions first.</p>
          {/each}
        </div>
      </section>

      <section>
        <h2 class="eyebrow mb-3">Settings</h2>
        <div class="panel divide-y">
          {#snippet modeControl()}<Seg label="Mode" bind:value={exam} options={[{ value: false, label: "Study" }, { value: true, label: "Exam" }]} />{/snippet}
          {@render row("Mode", "Study shows feedback after each question. Exam shows it at the end, and you can't go back.", modeControl)}

          {#snippet amountControl()}
            <div class="flex items-center gap-2">
              <Seg label="Amount" bind:value={setNumber} options={[{ value: false, label: "All" }, { value: true, label: "Set number" }]} />
              <input class="input input-sm w-20 tabular-nums" type="number" min="1" aria-label="Number of questions" bind:value={count} disabled={!setNumber} />
            </div>
          {/snippet}
          {@render row("Questions", "", amountControl)}

          {#snippet pickControl()}<Seg label="Pick" bind:value={even} options={[{ value: false, label: "Random" }, { value: true, label: "Even" }]} />{/snippet}
          {@render row("Pick", "Random draws from all topics together. Even takes the same number from each.", pickControl)}

          {#snippet questionOrder()}<Seg label="Question order" bind:value={shuffleQuestions} options={[{ value: false, label: "Keep", name: "Keep questions" }, { value: true, label: "Shuffle", name: "Shuffle questions" }]} />{/snippet}
          {@render row("Question order", "", questionOrder)}

          {#snippet optionOrder()}<Seg label="Option order" bind:value={shuffleOptions} options={[{ value: false, label: "Keep", name: "Keep options" }, { value: true, label: "Shuffle", name: "Shuffle options" }]} />{/snippet}
          {@render row("Option order", "", optionOrder)}

          {#snippet totalTimer()}
            <div class="flex items-center gap-2">
              <input type="checkbox" class="toggle toggle-sm" aria-label="Use total timer" bind:checked={useTotal} />
              <input class="input input-sm w-20 tabular-nums" type="number" min="1" aria-label="Minutes" bind:value={totalMinutes} disabled={!useTotal} /><span class="text-sm text-muted">min</span>
            </div>
          {/snippet}
          {@render row("Total timer", "", totalTimer)}

          {#snippet perQuestionTimer()}
            <div class="flex items-center gap-2">
              <input type="checkbox" class="toggle toggle-sm" aria-label="Use per-question timer" bind:checked={usePerQuestion} />
              <input class="input input-sm w-20 tabular-nums" type="number" min="5" aria-label="Seconds" bind:value={perQuestionSeconds} disabled={!usePerQuestion} /><span class="text-sm text-muted">sec</span>
            </div>
          {/snippet}
          {@render row("Per-question timer", "Running out counts as wrong and moves on.", perQuestionTimer)}

          {#snippet difficulty()}<Seg label="Difficulty" bind:value={easy} options={[{ value: true, label: "Easy" }, { value: false, label: "Hard" }]} />{/snippet}
          {@render row("Difficulty", "Easy shows how many answers to pick on multiple-answer questions.", difficulty)}
        </div>
      </section>
    </div>

    <aside class="panel sticky top-8 p-6" aria-label="Quiz summary">
      <p class="eyebrow">Your quiz</p>
      {#if chosen.length === 0}
        <p class="mt-4 text-sm text-muted">Pick a topic to see your quiz.</p>
      {:else}
        <p class="mt-3 text-4xl font-semibold leading-none">{questionCount} <span class="text-base font-normal text-muted">questions</span></p>
        <dl class="mt-6 space-y-3 text-sm">
          <div class="flex justify-between"><dt class="text-muted">Mode</dt><dd>{exam ? "Exam" : "Study"}</dd></div>
          <div class="flex justify-between"><dt class="text-muted">Time limit</dt><dd>{useTotal ? `${totalMinutes} min total` : usePerQuestion ? `${perQuestionSeconds} sec each` : "None"}</dd></div>
          <div class="flex justify-between"><dt class="text-muted">Order</dt><dd>{shuffleQuestions ? "Shuffled" : "As written"}</dd></div>
        </dl>
      {/if}
      {#if error}<p role="alert" class="mt-6 flex items-start gap-2 text-sm text-error"><CircleAlert class="mt-0.5 size-4 shrink-0" />{error}</p>{/if}
      <button class="btn btn-primary mt-6 w-full" onclick={start} disabled={starting}><Play class="size-4" />Start quiz</button>
      <p class="mt-3 text-center text-xs text-muted">Press <kbd class="kbd kbd-xs">1</kbd>–<kbd class="kbd kbd-xs">5</kbd> to answer, <kbd class="kbd kbd-xs">F</kbd> to flag a guess</p>
    </aside>
  </div>
</div>
