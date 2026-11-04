import { t, useLocale } from './i18n/locale';
import { ResearchProblemPage } from './pages/ResearchProblems';
import { ResearchGraphPage } from './pages/ResearchGraph';
import { FeedbackPage } from './pages/Feedback';
import { BindingsPage, BindingPage, BindingProfilePage } from './pages/Bindings';
import { FormalizationsPage } from './pages/Formalizations';
import { useEffect } from 'react';
import { SupportersPage, StatementPage } from './pages/Contributions';
import { ConjecturePage } from './pages/Conjecture';
import { LeaderboardPage, EntrantPage } from './pages/Leaderboard';
import { Link } from './routing/router';
import { Header } from './components/Header';
import { BrandMark } from './components/BrandMark';
import { ConjecturesPage } from './pages/Conjectures';
import { ContributePage } from './pages/Contribute';
import { ContributorsPage } from './pages/Contributors';
import { HomePage } from './pages/Home';
import { NotFoundPage } from './pages/NotFound';
import { ResultPage } from './pages/Result';
import { PaperPage, PapersPage } from './pages/Papers';
import { SubjectPage, SubjectsPage } from './pages/Subjects';
import { findSubject } from './lib/subjects';
import { PeoplePage } from './pages/People';
import { AuthorPage } from './pages/Author';
import { PolicyPage } from './pages/Policy';
import { SubmissionDetailPage } from './pages/SubmissionDetail';
import { SubmissionsPage } from './pages/Submissions';
import { SubmitPage } from './pages/Submit';
import { TaskPage, TasksPage } from './pages/Tasks';
import { useRouter } from './routing/router';
import { routePath, type Route } from './routing/routes';

function titleFor(route: Route): string {
  switch (route.kind) {
    case 'subjects':
      return 'Browse by subject';
    case 'subject':
      return findSubject(route.msc)?.title ?? 'Not found';
    case 'research_graph':
      return 'Research graph';
    case 'feedback':
      return 'Knowledge proposal';
    case 'binding_profile':
      return 'Binding contributions';
    case 'bindings':
      return 'Binding discussions';
    case 'binding':
      return 'Binding discussion';
    case 'supporters':
      return 'Supporters';
    case 'statement':
      return route.record;
    case 'home':
      return 'Mathesis';
    case 'leaderboard':
      return 'Leaderboard';
    case 'entrant':
      return 'Solver';
    case 'conjecture':
      return route.record;
    case 'formalizations':
      return 'Formalizations';
    case 'research_problem':
      return 'Research problem';
    case 'conjectures':
      return 'Conjectures';
    case 'papers':
      return 'Research archive';
    case 'result':
    case 'paper':
      return route.record;
    case 'policy':
      return 'Review policy';
    case 'submit':
      return 'Submit your work';
    case 'submissions':
      return 'My work';
    case 'queue':
      return 'Review queue';
    case 'submission':
      return 'Paper';
    case 'people':
      return 'People';
    case 'author':
      return route.name;
    case 'authors':
      return 'Authors';
    case 'contribute':
      return 'Contribute';
    case 'tasks':
      return 'Tasks';
    case 'task':
      return 'Task';
    case 'contributors':
      return 'Contributors';
    case 'not_found':
      return 'Not found';
  }
}

function Page({ route }: { route: Route }) {
  switch (route.kind) {
    case 'subjects':
      return <SubjectsPage />;
    case 'subject':
      return <SubjectPage key={route.msc} msc={route.msc} />;
    case 'research_graph':
      return (
        <ResearchGraphPage
          key={routePath(route)}
          focus={route.focus}
          q={route.q}
          before={route.before}
        />
      );
    case 'feedback':
      return <FeedbackPage key={routePath(route)} id={route.id} proposalKey={route.key} />;
    case 'binding_profile':
      return <BindingProfilePage key={route.person} person={route.person} />;
    case 'bindings':
      return <BindingsPage />;
    case 'binding':
      return <BindingPage key={routePath(route)} id={route.id} boardKey={route.key} />;
    case 'supporters':
      return <SupportersPage />;
    case 'statement':
      return <StatementPage record={route.record} claim={route.claim} />;
    case 'home':
      return <HomePage />;
    case 'leaderboard':
      return <LeaderboardPage />;
    case 'entrant':
      return <EntrantPage id={route.id} />;
    case 'conjecture':
      return <ConjecturePage record={route.record} claim={route.claim} />;
    case 'research_problem':
      return <ResearchProblemPage key={routePath(route)} id={route.id} before={route.before} />;
    case 'conjectures':
      return <ConjecturesPage />;
    case 'papers':
      return (
        <PapersPage
          key={routePath(route)}
          initialQuery={route.q}
          initialFilters={{ author: route.author, msc: route.msc, kind: route.work }}
        />
      );
    case 'result':
      return <ResultPage key={routePath(route)} record={route.record} claim={route.claim} />;
    case 'paper':
      return <PaperPage key={routePath(route)} record={route.record} version={route.version} />;
    case 'policy':
      return <PolicyPage />;
    case 'formalizations':
      return <FormalizationsPage />;
    case 'submit':
      return (
        <SubmitPage
          key={routePath(route)}
          initialIntake={route.intake}
          targetRecord={route.record}
          targetClaim={route.claim}
        />
      );
    case 'submissions':
      return <SubmissionsPage key="mine" scope="mine" />;
    case 'queue':
      return <SubmissionsPage key="queue" scope="queue" />;
    case 'submission':
      return (
        <SubmissionDetailPage
          key={`${route.id}-${route.staff}`}
          id={route.id}
          staffView={route.staff}
        />
      );
    case 'people':
      return <PeoplePage />;
    case 'author':
      return <AuthorPage name={route.name} />;
    case 'authors':
      return <PapersPage />;
    case 'contribute':
      return <ContributePage />;
    case 'tasks':
      return (
        <TasksPage
          key={routePath(route)}
          submission={route.submission}
          initialKind={route.taskKind}
        />
      );
    case 'task':
      return <TaskPage key={route.id} id={route.id} />;
    case 'contributors':
      return <ContributorsPage />;
    case 'not_found':
      return <NotFoundPage path={route.path} />;
  }
}

export function App() {
  const { route } = useRouter();
  const locale = useLocale();
  useEffect(() => {
    const title = route.kind === 'author' ? route.name : t(titleFor(route));
    document.title = title === 'Mathesis' ? 'Mathesis' : `${title} · Mathesis`;
  }, [route, locale]);
  return (
    <>
      <a className="skip-link" href="#main">
        {t('Skip to content')}
      </a>
      <Header />
      <main id="main">
        <Page route={route} />
      </main>
      <footer className="site-footer">
        <div className="site-footer-inner">
          <div className="footer-identity">
            <Link to={{ kind: 'home' }} className="wordmark">
              <BrandMark />
              Mathesis
            </Link>
            <p>{t('Mathematical research, review, and verified proofs.')}</p>
          </div>
          <nav aria-label={t('Footer')}>
            <Link to={{ kind: 'contribute' }}>{t('Contribute')}</Link>
            <Link to={{ kind: 'leaderboard' }}>{t('Leaderboard')}</Link>
            <Link to={{ kind: 'policy' }}>{t('Policy')}</Link>
            <Link to={{ kind: 'supporters' }}>{t('Supporters')}</Link>
          </nav>
          <span className="footer-note">
            {t('Read research. Share results. Publish questions.')}
          </span>
        </div>
      </footer>
    </>
  );
}
