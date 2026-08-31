import { Component, ErrorInfo, ReactNode } from 'react';
import { Alert, Button } from 'antd';

interface Props {
  children: ReactNode;
}

interface State {
  hasError: boolean;
  error: Error | null;
}

class ErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = {
      hasError: false,
      error: null,
    };
  }

  static getDerivedStateFromError(error: Error): State {
    return {
      hasError: true,
      error,
    };
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo): void {
    console.error('Error caught by ErrorBoundary:', error, errorInfo);
  }

  handleRetry = (): void => {
    this.setState({ hasError: false, error: null });
  };

  render(): ReactNode {
      if (this.state.hasError) {
        return (
          <div className="mc-error-boundary">
            <div className="mc-error-card">
              <Alert
                message="应用错误"
                description={this.state.error?.message || '发生了未知错误'}
                type="error"
                showIcon
                action={
                  <Button type="primary" size="small" onClick={this.handleRetry}>
                    重试
                  </Button>
                }
              />
              <p className="mc-error-desc">
                系统遇到了一个问题，我们已经记录了错误信息。
                请尝试刷新页面或联系管理员。
              </p>
            </div>
          </div>
        );
      }

    return this.props.children;
  }
}

export default ErrorBoundary;