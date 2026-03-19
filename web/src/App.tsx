import { useEffect } from 'react'
import { Route, Routes, Navigate, useLocation } from 'react-router-dom'
import { Header } from '@/components/layout/Header'
import { Footer } from '@/components/layout/Footer'
import { HomePage } from '@/components/pages/HomePage'
import { InstallPage } from '@/components/pages/InstallPage'
import { ExamplesPage } from '@/components/pages/ExamplesPage'
import { EditorPage } from '@/components/pages/EditorPage'
import { DocPage } from '@/components/pages/DocPage'
import { NotFoundPage } from '@/components/pages/NotFoundPage'

function ScrollToTop() {
  const { pathname } = useLocation()
  useEffect(() => { window.scrollTo(0, 0) }, [pathname])
  return null
}

export default function App() {
  return (
    <>
      <ScrollToTop />
      <Header />
      <Routes>
        <Route path="/" element={<HomePage />} />
        <Route path="/install" element={<InstallPage />} />
        <Route path="/examples" element={<ExamplesPage />} />
        <Route path="/editors/:editor" element={<EditorPage />} />
        <Route path="/docs" element={<Navigate to="/docs/getting-started" replace />} />
        <Route path="/docs/*" element={<DocPage />} />
        <Route path="*" element={<NotFoundPage />} />
      </Routes>
      <Footer />
    </>
  )
}
