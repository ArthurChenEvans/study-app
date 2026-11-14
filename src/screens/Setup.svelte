<script lang="ts">
  // quiz-setup.feature, short-answer.feature (R15 to R19, R36 to R38, R85)
  import { ChevronDown, CircleAlert, Play } from "@lucide/svelte";
  import { api, errorMessage, type Id, type ModuleRow, type QuestionTypes, type QuizView, type TopicQuizView, type TopicRow } from "../lib/api";
  import PageHeader from "../components/PageHeader.svelte";
  import Seg from "../components/Seg.svelte";
  import TopicPicker from "../components/list/TopicPicker.svelte";
  import { X } from "@lucide/svelte";

  let { preselected = [], onStarted }: { preselected?: Id[]; onStarted: (view: QuizView) => void } = $props();

  let topics = $state<TopicRow[]>([]);
  let modules = $state<ModuleRow[]>([]);
  let quizzes = $state<TopicQuizView[]>([]);
  // Settings fold away into one summary line: most quizzes use the same ones every time.
  let showSettings = $state(false);
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
  let types = $state<QuestionTypes>("Both");

  let error = $state("");
  let starting = $state(false);

  // R85: how many of the chosen type a topic has. Topics with none are left out.
  const countOf = (topic: TopicRow) => (types === "Both" ? topic.question_count : types === "Mcq" ? topic.question_count - topic.short_count : topic.short_count);
  const anyShort = $derived(topics.some((topic) => topic.short_count > 0));
  const included = $derived(topics.filter((topic) => chosen.includes(topic.id) && countOf(topic) > 0));
  const available = $derived(included.reduce((sum, topic) => sum + countOf(topic), 0));
  function countLabel(topic: TopicRow) {
    const mcq = topic.question_count - topic.short_count;
    if (types === "Short") return topic.short_count ? `${topic.short_count} short answer` : "no short answer yet";
    if (types === "Mcq") return mcq ? `${mcq} MCQ` : "no MCQs yet";
    if (topic.question_count === 0) return "no questions yet";
    return topic.short_count ? `${mcq} MCQ · ${topic.short_count} short answer` : `${topic.question_count} questions`;
  }
  // R36: asking for more than exist uses all of them.
  const questionCount = $derived(setNumber ? Math.min(Number(count) || 0, available) : available);
  const overAsked = $derived(setNumber && Number(count) > available && chosen.length > 0);
  const moduleName = (id: Id | null) => modules.find((module) => module.id === id)?.name ?? "No module";

  const isDue = (topic: TopicRow) => quizzes.some((quiz) => quiz.topic === topic.id && quiz.due_today) && countOf(topic) > 0;
  const dueIds = $derived(topics.filter(isDue).map((topic) => topic.id));
  const summary = $derived([
    ...(anyShort ? [types === "Both" ? "Both types" : types === "Mcq" ? "MCQ only" : "Short answer only"] : []),
    exam ? "Exam mode" : "Study mode",
    setNumber ? `${count} questions` : "All questions",
    even ? "Even pick" : "Random pick",
    shuffleQuestions ? "Shuffled" : "Keep order",
    useTotal ? `${totalMinutes} min total` : usePerQuestion ? `${perQuestionSeconds} sec each` : "No timer",
    easy ? "Easy" : "Hard",
  ]);

  function pickDue() {
    chosen = [...new Set([...chosen, ...dueIds])];
  }

  function toggle(id: Id) {
    chosen = chosen.includes(id) ? chosen.filter((topic) => topic !== id) : [...chosen, id];
  }

  async function start() {
    error = "";
    if (chosen.length === 0) {
      error = "Pick at least one topic to start.";
      return;
    }
    if (included.length === 0) {
      error = types === "Short" ? "None of these topics have short answer questions yet." : "None of these topics have multiple choice questions yet.";
      return;
    }
    starting = true;
    try {
      const view = await api.startQuiz({
        topics: included.map((topic) => topic.id),
        count: setNumber ? Number(count) : null,
        even,
        shuffle_questions: shuffleQuestions,
        shuffle_options: shuffleOptions,
        exam,
        total_minutes: useTotal ? Number(totalMinutes) : null,
        per_question_seconds: usePerQuestion ? Number(perQuestionSeconds) : null,
        easy,
        types,
      });
      onStarted(view);
    } catch (failure) {
      error = errorMessage(failure);
    } finally {
      starting = false;
    }
  }

  Promise.all([api.topics(), api.modules(), Promise.resolve().then(() => api.topicQuizzes()).catch(() => [])]).then(([topicRows, moduleRows, quizRows]) => {
    topics = topicRows;
    modules = moduleRows;
    quizzes = quizRows ?? [];
  });
</script>

{#snippet row(label: string, help: string, control: import("svelte").Snippet)}
  <div class="flex items-center justify-between gap-8 px-6 py-4">
    <div><p class="text-sm font-semibold">{label}</p>{#if help}<p class="mt-1 text-sm text-muted">{help}</p>{/if}</div>
    <div class="shrink-0">{@render control()}</div>
  </div>
{/snippet}

<div class="mx-auto max-w-[96rem] px-12 pb-24">
  <PageHeader title="New quiz" subtitle="Pick what to practise and how" />
  <div class="grid grid-cols-1 items-start gap-8 wide:grid-cols-[minmax(0,1fr)_320px]">
    <div class="space-y-8">
      <section>
        <div class="mb-3 flex items-center justify-between">
          <h2 class="eyebrow">Topics</h2>
          {#if dueIds.length && dueIds.some((id) => !chosen.includes(id))}<button class="btn btn-link btn-sm text-[13px]" onclick={pickDue}>Pick the {dueIds.length} due</button>{/if}
        </div>
        {#snippet rowEnd(topic: TopicRow)}
          {#if isDue(topic)}<span class="chip chip-due">Due today</span>{/if}
          <span class="shrink-0 text-[13px] tabular-nums text-muted">{countLabel(topic)}</span>
        {/snippet}
        <TopicPicker {topics} {modules} bind:chosen label="Topics to quiz" disabled={(topic) => countOf(topic) === 0} extra={rowEnd} />
      </section>

      <section>
        <h2 class="eyebrow mb-3">Settings</h2>
        <div class="panel flex flex-wrap items-center gap-2 px-6 py-4">
          <ul class="flex flex-1 flex-wrap gap-2" aria-label="Current settings">
            {#each summary as item (item)}<li class="word">{item}</li>{/each}
          </ul>
          <button class="btn btn-ghost btn-sm" aria-expanded={showSettings} aria-controls="quiz-settings" onclick={() => (showSettings = !showSettings)}>
            {showSettings ? "Hide settings" : "Change settings"}<ChevronDown class="size-4 transition-transform {showSettings ? 'rotate-180' : ''}" />
          </button>
        </div>
        {#if showSettings}
        <div id="quiz-settings" class="panel mt-3 divide-y">
          {#if anyShort}
            {#snippet typesControl()}<Seg label="Question types" bind:value={types} options={[{ value: "Both", label: "Both" }, { value: "Mcq", label: "MCQ" }, { value: "Short", label: "Short answer" }]} />{/snippet}
            {@render row("Question types", "Mixed together in the order below.", typesControl)}
          {/if}
          {#snippet modeControl()}<Seg label="Mode" bind:value={exam} options={[{ value: false, label: "Study" }, { value: true, label: "Exam" }]} />{/snippet}
          {@render row("Mode", "Study shows feedback after each question. Exam shows it at the end. You can't go back in either.", modeControl)}

          {#snippet amountControl()}
            <div class="flex items-center gap-2">
              <Seg label="Amount" bind:value={setNumber} options={[{ value: false, label: "All" }, { value: true, label: "Set number" }]} />
              <input class="input input-sm w-20 tabular-nums" type="number" min="1" aria-label="Number of questions" bind:value={count} disabled={!setNumber} />
            </div>
            {#if overAsked}<p class="mt-1 text-right text-xs text-muted">Only {available} available</p>{/if}
          {/snippet}
          {@render row("Questions", "", amountControl)}

          {#snippet pickControl()}<Seg label="Pick" bind:value={even} options={[{ value: false, label: "Random" }, { value: true, label: "Even" }]} />{/snippet}
          {@render row("Pick", "Random draws from all topics together. Even takes the same number from each, giving any extra to the first topics.", pickControl)}

          {#snippet questionOrder()}<Seg label="Question order" bind:value={shuffleQuestions} options={[{ value: false, label: "Keep", name: "Keep questions" }, { value: true, label: "Shuffle", name: "Shuffle questions" }]} />{/snippet}
          {@render row("Question order", "", questionOrder)}

          {#snippet optionOrder()}<Seg label="Option order" bind:value={shuffleOptions} options={[{ value: false, label: "Keep", name: "Keep options" }, { value: true, label: "Shuffle", name: "Shuffle options" }]} />{/snippet}
          {@render row("Option order", anyShort ? "Multiple choice only." : "", optionOrder)}

          {#snippet totalTimer()}
            <div class="flex items-center gap-2">
              <input type="checkbox" class="toggle" aria-label="Use total timer" bind:checked={useTotal} />
              <input class="input input-sm w-20 tabular-nums" type="number" min="1" aria-label="Minutes" bind:value={totalMinutes} disabled={!useTotal} /><span class="text-sm text-muted">min</span>
            </div>
          {/snippet}
          {@render row("Total timer", "", totalTimer)}

          {#snippet perQuestionTimer()}
            <div class="flex items-center gap-2">
              <input type="checkbox" class="toggle" aria-label="Use per-question timer" bind:checked={usePerQuestion} />
              <input class="input input-sm w-20 tabular-nums" type="number" min="5" aria-label="Seconds" bind:value={perQuestionSeconds} disabled={!usePerQuestion} /><span class="text-sm text-muted">sec</span>
            </div>
          {/snippet}
          {@render row("Per-question timer", "Running out counts as wrong and moves on.", perQuestionTimer)}

          {#snippet difficulty()}<Seg label="Difficulty" bind:value={easy} options={[{ value: true, label: "Easy" }, { value: false, label: "Hard" }]} />{/snippet}
          {@render row("Difficulty", `Easy shows how many answers to pick on multiple-answer questions.${anyShort ? " Multiple choice only." : ""}`, difficulty)}
        </div>
        {/if}
      </section>
    </div>

    <aside class="panel sticky top-8 p-6" aria-label="Quiz summary">
      <p class="eyebrow">Your quiz</p>
      {#if chosen.length === 0}
        <p class="mt-4 text-sm text-muted">No topics picked yet.</p>
      {:else}
        <p class="mt-3 text-4xl font-semibold leading-none">{questionCount} <span class="text-base font-normal text-muted">questions</span></p>
        <p class="mt-1 text-sm text-muted">from {chosen.length} {chosen.length === 1 ? "topic" : "topics"}</p>
        <ul class="mt-3 flex max-h-40 flex-wrap gap-1.5 overflow-y-auto" aria-label="Picked topics">
          {#each topics.filter((t) => chosen.includes(t.id)) as topic (topic.id)}
            <li><button type="button" class="chip chip-primary min-h-6" aria-label="Untick {topic.name}" onclick={() => toggle(topic.id)}>{topic.name}<X class="size-3" aria-hidden="true" /></button></li>
          {/each}
        </ul>
        <dl class="mt-6 space-y-3 text-sm">
          {#if anyShort}<div class="flex justify-between"><dt class="text-muted">Types</dt><dd>{types === "Both" ? "MCQ and short answer" : types === "Mcq" ? "MCQ only" : "Short answer only"}</dd></div>{/if}
          <div class="flex justify-between"><dt class="text-muted">Mode</dt><dd>{exam ? "Exam" : "Study"}</dd></div>
          <div class="flex justify-between"><dt class="text-muted">Time limit</dt><dd>{useTotal ? `${totalMinutes} min total` : usePerQuestion ? `${perQuestionSeconds} sec each` : "None"}</dd></div>
          <div class="flex justify-between"><dt class="text-muted">Order</dt><dd>{shuffleQuestions ? "Shuffled" : "As written"}</dd></div>
        </dl>
      {/if}
      {#if error}<p role="alert" class="mt-6 flex items-start gap-2 text-sm text-error"><CircleAlert class="mt-0.5 size-4 shrink-0" />{error}</p>{/if}
      <!-- aria-disabled rather than disabled: it stays focusable, and the reason is read with it. -->
      <button class="btn btn-primary mt-6 w-full {chosen.length === 0 ? 'btn-soft-off' : ''}" onclick={start} disabled={starting} aria-disabled={chosen.length === 0} aria-describedby={chosen.length === 0 ? "start-why" : undefined}><Play class="size-4" />Start quiz</button>
      {#if chosen.length === 0 && !error}<p id="start-why" class="mt-2 text-center text-sm text-muted">Pick at least one topic to start.</p>{/if}
      <p class="mt-3 text-center text-xs text-muted">In the quiz, press <kbd class="kbd kbd-xs">1</kbd>–<kbd class="kbd kbd-xs">5</kbd> to answer, <kbd class="kbd kbd-xs">F</kbd> to flag a guess</p>
    </aside>
  </div>
</div>
