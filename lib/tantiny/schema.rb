# frozen_string_literal: true

module Tantiny
  class Schema
    Field = Struct.new(
      'Field',
      :type, :key, :stored, :tokenizer,
      keyword_init: true
    ) do
      self::TYPES = %i[text string integer double date facet]

      def initialize(
          type: nil,
          key: nil,
          stored: false,
          tokenizer: nil)
        super
      end

      def text?
        type == :text
      end

      def stored? = stored
    end

    Field::TYPES.each do |sym|
      define_method "#{sym}_fields" do
        @fields.values.filter { _1.type == sym }
      end
    end

    def field_tokenizers
      text_fields.filter_map(&:tokenizer)
    end

    attr_reader :default_tokenizer,
      :id_field,
      :field_tokenizers,
      :fields

    def initialize(tokenizer, &block)
      @default_tokenizer = tokenizer
      @id_field = :id

      @fields = {}

      instance_exec(&block)
    end

    def tokenizer_for(key)
      field = @fields[key]
      return nil unless field&.text?

      field.tokenizer || default_tokenizer
    end

    private

    def id(key) = @id_field = key

    def field(type, key, **options)
      @fields[key] = Field.new(type:, key:, **options)
    end

    def text(key, tokenizer: nil, **options)
      field(:text, key, tokenizer:, **options)
    end

    def string(key, **options)
      field(:string, key, **options)
    end

    def integer(key, **options)
      field(:integer, key, **options)
    end

    def double(key, **options)
      field(:double, key, **options)
    end

    def date(key, **options)
      field(:date, key, **options)
    end

    def facet(key, **options)
      field(:facet, key, **options)
    end
  end
end
