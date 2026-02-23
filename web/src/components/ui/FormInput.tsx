interface FormInputProps {
  value: string;
  onChange: (value: string) => void;
  type?: 'text' | 'password' | 'email' | 'number' | 'url';
  placeholder?: string;
  disabled?: boolean;
  error?: string;
  hint?: string;
  className?: string;
  autoComplete?: string;
}

export function FormInput({
  value,
  onChange,
  type = 'text',
  placeholder,
  disabled = false,
  error,
  hint,
  className = '',
  autoComplete,
}: FormInputProps) {
  return (
    <div className="space-y-1.5">
      <input
        type={type}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
        disabled={disabled}
        autoComplete={autoComplete}
        className={`
          w-full bg-gray-900 border text-gray-200 text-sm rounded-lg px-3 py-2.5
          focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-inset
          transition-colors placeholder:text-gray-500
          ${error ? 'border-red-500 focus:ring-red-500' : 'border-gray-700 hover:border-gray-600'}
          ${disabled ? 'opacity-50 cursor-not-allowed' : ''}
          ${className}
        `}
      />
      {hint && !error && (
        <p className="text-xs text-gray-500">{hint}</p>
      )}
      {error && (
        <p className="text-xs text-red-400">{error}</p>
      )}
    </div>
  );
}
