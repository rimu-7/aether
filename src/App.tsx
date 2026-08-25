import { Routes, Route, Navigate } from "react-router-dom";
import { ThemeProvider } from "./components/theme-provider";
import "./App.css";

import { AppLayout } from "./components/layout/AppLayout";
import { Dashboard } from "./features/dashboard/Dashboard";
import { Applications } from "./features/applications/Applications";
import { Packages } from "./features/packages/Packages";
import { Cleaner } from "./features/cleaner/Cleaner";
import { Files } from "./features/files/Files";

function App() {
  return (
    <ThemeProvider defaultTheme="system">
      <Routes>
        <Route path="/" element={<AppLayout />}>
          <Route index element={<Dashboard />} />
          <Route path="applications" element={<Applications />} />
          <Route path="packages" element={<Packages />} />
          <Route path="cleaner" element={<Cleaner />} />
          <Route path="activity" element={<Files />} />
          <Route path="*" element={<Navigate to="/" replace />} />
        </Route>
      </Routes>
    </ThemeProvider>
  );
}

export default App;
